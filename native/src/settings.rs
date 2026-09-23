pub fn skin_picture(folder: &Path, side: u32) -> Option<Vec<u8>> {
    let sprites = skin_sprites(folder);
    let mut made = image::RgbaImage::new(side, side);
    let reach = (side as f32 * 0.84) as u32;
    let colour = first_combo(folder);
    match sprite_of(&sprites, Element::HitCircle) {
        Some(circle) => {
            let circle = match colour {
                Some(colour) => image::DynamicImage::ImageRgba8(tinted(circle.to_rgba8(), colour)),
                None => circle,
            };
            laid(&mut made, &circle, reach);
        }
        None => laid(&mut made, &image::DynamicImage::ImageRgba8(drawn_circle(side * 2, Some(colour.unwrap_or([255, 192, 0])), 0.22)), reach),
    }
    if let Some(over) = sprite_of(&sprites, Element::HitCircleOverlay) {
        laid(&mut made, &over, reach);
    }
    if let Some(number) = sprite_of(&sprites, Element::Digit(1)) {
        laid(&mut made, &number, (side as f32 * 0.3) as u32);
    }
    Some(made.into_raw())
}

use std::path::{Path, PathBuf};

use crate::lang::Lang;
use crate::sources::Source;

pub const DEFAULT_SERVER: &str = "https://onenineeightfour.ignorelist.com";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub sources: Vec<Source>,
    pub device: String,
    pub server: String,
    pub token: String,
    pub linked_as: String,
    #[serde(default)]
    pub menu_tab: String,
    #[serde(default = "yes")]
    pub live_scene: bool,
    #[serde(default = "yes")]
    pub pause_unfocused: bool,
    #[serde(default = "default_height")]
    pub render_height: u32,
    #[serde(default = "default_fps")]
    pub render_fps: u32,
    #[serde(default = "default_crf")]
    pub render_crf: u32,
    #[serde(default)]
    pub renders_dir: Option<PathBuf>,
    #[serde(default)]
    pub settings_tab: String,
    #[serde(default)]
    pub tiles_app: Vec<String>,
    #[serde(default)]
    pub tiles_bot: Vec<String>,
    #[serde(default)]
    pub chat_id: Option<i64>,
    #[serde(default)]
    pub tell: Tell,
    #[serde(default)]
    pub skin: Option<PathBuf>,
    #[serde(default)]
    pub own_skins: Vec<PathBuf>,
    #[serde(default = "full")]
    pub music_level: f32,
    #[serde(default = "full")]
    pub hitsound_level: f32,
    #[serde(default = "full")]
    pub player_level: f32,
    #[serde(default)]
    pub player_muted: bool,
    #[serde(default = "full")]
    pub player_rate: f32,
    #[serde(default)]
    pub player_loop: bool,
    #[serde(default = "yes")]
    pub hud: bool,
    #[serde(default)]
    pub cursor_grows: bool,
    #[serde(default = "default_dim")]
    pub background_dim: f32,
    #[serde(default = "full")]
    pub background_blur: f32,
    #[serde(default = "yes")]
    pub map_sounds: bool,
    #[serde(default = "yes")]
    pub skin_sounds: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tell {
    pub rendered: bool,
    pub errors: bool,
    pub maps: bool,
    pub worker: bool,
}

impl Default for Tell {
    fn default() -> Tell {
        Tell { rendered: true, errors: false, maps: false, worker: true }
    }
}

fn yes() -> bool {
    true
}

fn default_height() -> u32 {
    1080
}

fn default_fps() -> u32 {
    60
}

fn default_crf() -> u32 {
    20
}

fn full() -> f32 {
    1.0
}

fn default_dim() -> f32 {
    0.82
}

pub const HEIGHTS: [u32; 5] = [480, 720, 1080, 1440, 2160];
pub const RATES: [u32; 4] = [24, 30, 60, 120];
pub const CRFS: [u32; 5] = [26, 23, 20, 17, 14];

pub fn skins_root() -> PathBuf {
    crate::sources::own_root().join("Skins")
}

pub fn skins_in(sources: &[Source], own: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = sources.iter().filter_map(|source| source.skins.clone()).collect();
    for found in crate::sources::find() {
        if let Some(skins) = found.skins {
            roots.push(skins);
        }
    }
    roots.push(skins_root());
    roots.sort();
    roots.dedup();
    for source in sources {
        if let Some(exports) = source.root.join("exports").is_dir().then(|| source.root.join("exports")) {
            roots.push(exports);
        }
    }
    let mut found: Vec<PathBuf> = own.iter().filter(|path| path.is_dir() || is_skin_file(path)).cloned().collect();
    for root in roots {
        let Ok(read) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() && !found.contains(&path) {
                found.push(path);
            } else if kind.is_file() && is_skin_file(&path) && !found.contains(&path) {
                found.push(path);
            }
        }
    }
    found.retain(|path| !is_skin_file(path) || !already_unpacked(path));
    found.sort_by_key(|path| skin_name(path).to_lowercase());
    found.truncate(80);
    found
}

const SKIN_MARKS: [&str; 14] = [
    "hitcircle.png",
    "hitcircleoverlay.png",
    "approachcircle.png",
    "cursor.png",
    "cursortrail.png",
    "hit300.png",
    "hit0.png",
    "sliderb0.png",
    "sliderfollowcircle.png",
    "followpoint.png",
    "spinner-circle.png",
    "menu-back.png",
    "scorebar-bg.png",
    "default-0.png",
];

const SKIPPED: [&str; 16] = [
    "Library",
    "Applications",
    "System",
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "$Recycle.Bin",
    "node_modules",
    "target",
    "build",
    "vendor",
    "Songs",
    "Replays",
    "Screenshots",
    "Data",
];

pub fn is_skin_file(path: &Path) -> bool {
    path.extension().is_some_and(|kind| kind.eq_ignore_ascii_case("osk"))
}

pub fn looks_like_skin(folder: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(folder) else {
        return false;
    };
    let mut marks = 0;
    let mut said = false;
    for entry in read.flatten().take(600) {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name == "skin.ini" {
            said = true;
        } else if name.ends_with(".osu") || name.ends_with(".osz") {
            return false;
        } else if SKIN_MARKS.contains(&name.replace("@2x", "").as_str()) {
            marks += 1;
        }
    }
    said || marks >= 3
}

pub fn already_unpacked(file: &Path) -> bool {
    let name = skin_name(file);
    let near = file.parent().map(|folder| folder.join(&name));
    [near, Some(skins_root().join(&name))]
        .into_iter()
        .flatten()
        .any(|folder| folder.is_dir() && looks_like_skin(&folder))
}

pub fn skin_name(path: &Path) -> String {
    let stem = match is_skin_file(path) {
        true => path.file_stem(),
        false => path.file_name(),
    };
    stem.map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
}

pub fn unpack_skin(file: &Path) -> Result<PathBuf, String> {
    unpack_skin_into(file, &skins_root())
}

pub fn unpack_skin_into(file: &Path, root: &Path) -> Result<PathBuf, String> {
    let name = skin_name(file);
    if name.is_empty() {
        return Err(file.display().to_string());
    }
    let into = root.join(&name);
    if into.is_dir() && looks_like_skin(&into) {
        return Ok(into);
    }
    let bytes = std::fs::read(file).map_err(|why| format!("{}: {why}", file.display()))?;
    std::fs::create_dir_all(&into).map_err(|why| format!("{}: {why}", into.display()))?;
    dossier_produce::skin::unpack(&bytes, &into)?;
    lift_lonely_folder(&into);
    Ok(into)
}

fn lift_lonely_folder(root: &Path) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    let inside: Vec<PathBuf> = read.flatten().map(|entry| entry.path()).collect();
    let [only] = inside.as_slice() else {
        return;
    };
    if !only.is_dir() {
        return;
    }
    let Ok(read) = std::fs::read_dir(only) else {
        return;
    };
    for entry in read.flatten() {
        let _ = std::fs::rename(entry.path(), root.join(entry.file_name()));
    }
    let _ = std::fs::remove_dir(only);
}

pub fn adopt_skin_files(root: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut made = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if is_skin_file(&path) {
            if let Ok(folder) = unpack_skin_into(&path, root) {
                made.push(folder);
            }
        }
    }
    made
}

pub fn hunt_skins(sources: &[Source], own: &[PathBuf]) -> Vec<PathBuf> {
    let root = skins_root();
    let _ = adopt_skin_files(&root);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    let mut walked = 0usize;
    let roots = hunt_roots();
    let mut queue: std::collections::VecDeque<(PathBuf, u32)> = roots.iter().cloned().map(|at| (at, 0)).collect();
    while let Some((folder, deep)) = queue.pop_front() {
        if walked > 20_000 || std::time::Instant::now() > deadline {
            break;
        }
        walked += 1;
        let Ok(read) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_file() {
                if is_skin_file(&path) && !already_unpacked(&path) {
                    let _ = unpack_skin_into(&path, &root);
                }
                continue;
            }
            if !kind.is_dir() || deep >= 4 {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || SKIPPED.iter().any(|skip| skip.eq_ignore_ascii_case(&name)) {
                continue;
            }
            queue.push_back((path, deep + 1));
        }
    }
    skins_in(sources, own)
}

fn hunt_roots() -> Vec<PathBuf> {
    let home = crate::sources::home();
    let mut roots = vec![home.clone()];
    for near in ["Downloads", "Desktop", "Documents", "Games", "osu!"] {
        let path = home.join(near);
        if path.is_dir() {
            roots.push(path);
        }
    }
    if let Ok(read) = std::fs::read_dir("/Volumes") {
        for entry in read.flatten() {
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                roots.push(entry.path());
            }
        }
    }
    for letter in 'D'..='H' {
        let drive = PathBuf::from(format!("{letter}:\\"));
        if drive.is_dir() {
            roots.push(drive);
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

pub fn skin_face(folder: &Path) -> Option<PathBuf> {
    for name in ["hitcircle@2x.png", "hitcircle.png", "cursor@2x.png", "cursor.png", "menu-background.jpg", "menu-background.png"] {
        let file = folder.join(name);
        if file.is_file() {
            return Some(file);
        }
    }
    None
}

use dossier_render::elements::{Element, Verdict};
use dossier_render::imported::Sprites;

const PATTERN_PARTS: [Element; 15] = [
    Element::HitCircle,
    Element::HitCircleOverlay,
    Element::SliderHead,
    Element::SliderHeadOverlay,
    Element::SliderTail,
    Element::SliderTailOverlay,
    Element::ApproachCircle,
    Element::Cursor,
    Element::CursorMiddle,
    Element::CursorTrail,
    Element::Digit(1),
    Element::Digit(2),
    Element::Digit(3),
    Element::Digit(4),
    Element::Verdict(Verdict::Three),
];

pub fn skin_sprites(folder: &Path) -> Sprites {
    Sprites::read(folder, &PATTERN_PARTS)
}

fn sprite_of(sprites: &Sprites, element: Element) -> Option<image::DynamicImage> {
    let sprite = sprites.get(element)?;
    let pixmap = &sprite.pixmap;
    let mut made = image::RgbaImage::new(pixmap.width(), pixmap.height());
    for (under, over) in made.pixels_mut().zip(pixmap.pixels()) {
        let straight = over.demultiply();
        *under = image::Rgba([straight.red(), straight.green(), straight.blue(), straight.alpha()]);
    }
    Some(image::DynamicImage::ImageRgba8(made))
}

pub fn combo_colours(folder: &Path) -> Vec<[u8; 3]> {
    let Ok(bytes) = std::fs::read(folder.join("skin.ini")) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        let line = line.trim();
        for which in 1..=8 {
            let Some(rest) = line.strip_prefix(&format!("Combo{which}")) else {
                continue;
            };
            let Some(values) = rest.trim_start().strip_prefix(':') else {
                continue;
            };
            let parts: Vec<u8> = values.split(',').filter_map(|part| part.trim().parse().ok()).collect();
            if let [r, g, b, ..] = parts.as_slice() {
                found.push([*r, *g, *b]);
            }
        }
    }
    found
}

pub fn first_combo(folder: &Path) -> Option<[u8; 3]> {
    let bytes = std::fs::read(folder.join("skin.ini")).ok()?;
    for line in String::from_utf8_lossy(&bytes).lines() {
        let Some(rest) = line.trim().strip_prefix("Combo1") else {
            continue;
        };
        let Some(values) = rest.trim_start().strip_prefix(':') else {
            continue;
        };
        let parts: Vec<u8> = values.split(',').filter_map(|part| part.trim().parse().ok()).collect();
        if let [r, g, b, ..] = parts.as_slice() {
            return Some([*r, *g, *b]);
        }
    }
    None
}

fn tinted(mut picture: image::RgbaImage, colour: [u8; 3]) -> image::RgbaImage {
    for pixel in picture.pixels_mut() {
        for (channel, shade) in pixel.0.iter_mut().take(3).zip(colour) {
            *channel = ((*channel as u16 * shade as u16) / 255) as u8;
        }
    }
    picture
}

fn laid(under: &mut image::RgbaImage, part: &image::DynamicImage, reach: u32) {
    let (middle_x, middle_y) = (under.width() as f32 / 2.0, under.height() as f32 / 2.0);
    laid_at(under, part, reach, middle_x, middle_y, 1.0);
}

fn laid_at(under: &mut image::RgbaImage, part: &image::DynamicImage, reach: u32, at_x: f32, at_y: f32, alpha: f32) {
    let (wide, high) = (part.width().max(1), part.height().max(1));
    let scale = reach as f32 / wide.max(high) as f32;
    let (w, h) = (((wide as f32 * scale) as u32).max(1), ((high as f32 * scale) as u32).max(1));
    let mut scaled = image::imageops::resize(part, w, h, image::imageops::FilterType::Lanczos3);
    if alpha < 1.0 {
        for pixel in scaled.pixels_mut() {
            pixel.0[3] = (pixel.0[3] as f32 * alpha) as u8;
        }
    }
    let x = (at_x - w as f32 / 2.0).round() as i64;
    let y = (at_y - h as f32 / 2.0).round() as i64;
    image::imageops::overlay(under, &scaled, x, y);
}

fn drawn_circle(side: u32, body: Option<[u8; 3]>, thin: f32) -> image::RgbaImage {
    let mut made = image::RgbaImage::new(side, side);
    let middle = side as f32 / 2.0;
    let outer = side as f32 / 2.0 - 1.0;
    let border = outer * thin;
    for (x, y, pixel) in made.enumerate_pixels_mut() {
        let away = ((x as f32 + 0.5 - middle).powi(2) + (y as f32 + 0.5 - middle).powi(2)).sqrt();
        let inside = (outer - away + 0.5).clamp(0.0, 1.0);
        if inside <= 0.0 {
            continue;
        }
        let ring = (away - (outer - border) + 0.5).clamp(0.0, 1.0);
        match body {
            Some(body) => {
                let shade: Vec<u8> = body.iter().map(|part| (*part as f32 + (255.0 - *part as f32) * ring) as u8).collect();
                *pixel = image::Rgba([shade[0], shade[1], shade[2], (inside * 255.0) as u8]);
            }
            None => *pixel = image::Rgba([255, 255, 255, (inside * ring * 255.0) as u8]),
        }
    }
    made
}

const FIELD: (f32, f32, f32, f32) = (0.0, 36.0, 512.0, 356.0);
const NOTE_RADIUS: f32 = 42.0;
const CURSOR_SCALE: f32 = 0.72;

fn slider_road(under: &mut image::RgbaImage, road: &[(f32, f32)], radius: f32, border: [u8; 3], scale: impl Fn(f32, f32) -> (f32, f32)) {
    let points: Vec<(f32, f32)> = road.iter().map(|(x, y)| scale(*x, *y)).collect();
    if points.len() < 2 {
        return;
    }
    let reach = radius;
    let rim = reach * 0.14;
    let (wide, high) = (under.width() as i32, under.height() as i32);
    let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in &points {
        left = left.min(x - reach);
        right = right.max(x + reach);
        top = top.min(y - reach);
        bottom = bottom.max(y + reach);
    }
    for y in (top.floor().max(0.0) as i32)..=(bottom.ceil().min(high as f32 - 1.0) as i32) {
        for x in (left.floor().max(0.0) as i32)..=(right.ceil().min(wide as f32 - 1.0) as i32) {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut away = f32::MAX;
            for pair in points.windows(2) {
                let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
                let (dx, dy) = (bx - ax, by - ay);
                let len = (dx * dx + dy * dy).max(0.0001);
                let t = (((px - ax) * dx + (py - ay) * dy) / len).clamp(0.0, 1.0);
                let (cx, cy) = (ax + dx * t, ay + dy * t);
                away = away.min(((px - cx).powi(2) + (py - cy).powi(2)).sqrt());
            }
            if away > reach {
                continue;
            }
            let edge = (reach - away + 0.5).clamp(0.0, 1.0);
            let inner = reach - rim;
            let shade = if away > inner {
                image::Rgba([border[0], border[1], border[2], (edge * 235.0) as u8])
            } else {
                let deep = (away / inner).clamp(0.0, 1.0);
                let body = 26.0 + 26.0 * (1.0 - deep);
                image::Rgba([body as u8, body as u8, (body + 6.0) as u8, 226])
            };
            let under_pixel = under.get_pixel_mut(x as u32, y as u32);
            let over = shade.0[3] as f32 / 255.0;
            for at in 0..3 {
                under_pixel.0[at] = (shade.0[at] as f32 * over + under_pixel.0[at] as f32 * (1.0 - over)) as u8;
            }
            under_pixel.0[3] = (255.0 * over + under_pixel.0[3] as f32 * (1.0 - over)).min(255.0) as u8;
        }
    }
}

fn slider_rim(folder: Option<&Path>) -> [u8; 3] {
    let Some(folder) = folder else {
        return [255, 255, 255];
    };
    let Ok(bytes) = std::fs::read(folder.join("skin.ini")) else {
        return [255, 255, 255];
    };
    for line in String::from_utf8_lossy(&bytes).lines() {
        let Some(rest) = line.trim().strip_prefix("SliderBorder") else {
            continue;
        };
        let Some(values) = rest.trim_start().strip_prefix(':') else {
            continue;
        };
        let parts: Vec<u8> = values.split(',').filter_map(|part| part.trim().parse().ok()).collect();
        if let [r, g, b, ..] = parts.as_slice() {
            return [*r, *g, *b];
        }
    }
    [255, 255, 255]
}

#[derive(Clone, Copy, PartialEq)]
enum Face {
    Note,
    Head,
    Tail,
}

pub fn skin_pattern(folder: Option<&Path>, wide: u32, high: u32) -> Vec<u8> {
    let mut made = image::RgbaImage::new(wide, high);
    let sprites = folder.map(skin_sprites);
    let speaks = |element: Element| sprites.as_ref().is_some_and(|have| !have.draw_ourselves(element));
    let part = |element: Element| sprites.as_ref().and_then(|have| sprite_of(have, element));
    let natural = |element: Element| {
        sprites
            .as_ref()
            .and_then(|have| have.get(element))
            .map(|sprite| sprite.pixmap.width().max(sprite.pixmap.height()) as f32 / sprite.scale.max(1.0))
    };
    let above_number = sprites.as_ref().is_none_or(|have| have.ini().overlay_above_number);
    let colours = folder.map(combo_colours).unwrap_or_default();
    let fallback: Vec<[u8; 3]> = dossier_beatmap::DEFAULT_COMBO_COLOURS.iter().map(|c| [c.r, c.g, c.b]).collect();
    let mine = |at: usize| -> Option<[u8; 3]> {
        match (folder.is_some(), colours.is_empty()) {
            (true, true) => None,
            (true, false) => Some(colours[at % colours.len()]),
            (false, _) => Some(fallback[at % fallback.len()]),
        }
    };
    let (fx, fy, fr, fb) = FIELD;
    let scale = (wide as f32 / (fr - fx)).min(high as f32 / (fb - fy));
    let (left, top) = (
        (wide as f32 - (fr - fx) * scale) / 2.0 - fx * scale,
        (high as f32 - (fb - fy) * scale) / 2.0 - fy * scale,
    );
    let place = |x: f32, y: f32| (left + x * scale, top + y * scale);
    let radius = NOTE_RADIUS * scale;
    let rim = slider_rim(folder);
    slider_road(&mut made, &[(132.0, 300.0), (420.0, 300.0)], radius, rim, place);
    slider_road(&mut made, &[(190.0, 206.0), (276.0, 206.0)], radius, rim, place);
    let face_of = |face: Face| -> Option<(Element, Element)> {
        let own = match face {
            Face::Note => None,
            Face::Head => Some((Element::SliderHead, Element::SliderHeadOverlay)),
            Face::Tail => Some((Element::SliderTail, Element::SliderTailOverlay)),
        };
        if let Some(pair) = own.filter(|pair| speaks(pair.0)) {
            return Some(pair);
        }
        speaks(Element::HitCircle).then_some((Element::HitCircle, Element::HitCircleOverlay))
    };
    let notes = [
        (150.0, 112.0, Some(1u8), 0usize, None, Face::Note),
        (246.0, 112.0, Some(2), 0, None, Face::Note),
        (342.0, 112.0, Some(3), 0, None, Face::Note),
        (438.0, 112.0, Some(4), 0, Some(1.55), Face::Note),
        (132.0, 300.0, Some(1), 1, None, Face::Head),
        (420.0, 300.0, None, 1, None, Face::Tail),
        (190.0, 206.0, Some(2), 1, None, Face::Head),
        (276.0, 206.0, None, 1, None, Face::Tail),
    ];
    for (x, y, number, combo, approach, face) in notes {
        let (cx, cy) = place(x, y);
        let body = mine(combo);
        let tinted_part = |element: Element| {
            part(element).map(|drawing| match body {
                Some(colour) => image::DynamicImage::ImageRgba8(tinted(drawing.to_rgba8(), colour)),
                None => drawing,
            })
        };
        match face_of(face) {
            Some((disc, overlay)) => {
                if let Some(drawing) = tinted_part(disc) {
                    laid_at(&mut made, &drawing, (radius * 2.0) as u32, cx, cy, 1.0);
                }
                let over = part(overlay);
                if !above_number {
                    if let Some(over) = &over {
                        laid_at(&mut made, over, (radius * 2.0) as u32, cx, cy, 1.0);
                    }
                }
                if let Some(number) = number {
                    if let (Some(drawing), Some(size)) = (part(Element::Digit(number)), natural(Element::Digit(number))) {
                        laid_at(&mut made, &drawing, (size * radius / 64.0 * 0.8).max(4.0) as u32, cx, cy, 1.0);
                    }
                }
                if above_number {
                    if let Some(over) = &over {
                        laid_at(&mut made, over, (radius * 2.0) as u32, cx, cy, 1.0);
                    }
                }
            }
            None if face == Face::Tail => {}
            None => {
                let own = image::DynamicImage::ImageRgba8(drawn_circle((radius * 4.0) as u32, Some(body.unwrap_or([255, 192, 0])), 0.22));
                laid_at(&mut made, &own, (radius * 2.0) as u32, cx, cy, 1.0);
            }
        }
        if let Some(reach) = approach {
            match speaks(Element::ApproachCircle) {
                true => {
                    if let Some(drawing) = tinted_part(Element::ApproachCircle) {
                        laid_at(&mut made, &drawing, (radius * 2.0 * reach) as u32, cx, cy, 0.85);
                    }
                }
                false => {
                    let ring = image::DynamicImage::ImageRgba8(drawn_circle((radius * 4.0) as u32, None, 0.045));
                    laid_at(&mut made, &ring, (radius * 2.0 * reach) as u32, cx, cy, 0.85);
                }
            }
        }
    }
    let (hand_x, hand_y) = place(70.0, 176.0);
    let on_screen = |element: Element| natural(element).map(|size| (size * scale * CURSOR_SCALE).max(3.0) as u32);
    if let (Some(trail), Some(size)) = (part(Element::CursorTrail), on_screen(Element::CursorTrail)) {
        for (step, away) in [(1.0, 0.42), (2.0, 0.28), (3.0, 0.18), (4.0, 0.1)] {
            let (tx, ty) = place(70.0 + 4.0 * step, 176.0 + 32.0 * step);
            laid_at(&mut made, &trail, size, tx, ty, away);
        }
    }
    match speaks(Element::Cursor) {
        true => {
            if let (Some(drawing), Some(size)) = (part(Element::Cursor), on_screen(Element::Cursor)) {
                laid_at(&mut made, &drawing, size, hand_x, hand_y, 1.0);
            }
            if let (Some(middle), Some(size)) = (part(Element::CursorMiddle), on_screen(Element::CursorMiddle)) {
                laid_at(&mut made, &middle, size, hand_x, hand_y, 1.0);
            }
        }
        false => {
            let dot = image::DynamicImage::ImageRgba8(drawn_circle((radius * 2.0) as u32, Some([255, 255, 255]), 0.42));
            laid_at(&mut made, &dot, (radius * 0.62) as u32, hand_x, hand_y, 1.0);
        }
    }
    made.into_raw()
}

pub fn skins_under(folder: &Path) -> Vec<PathBuf> {
    let mut found = adopt_skin_files(folder);
    if let Ok(read) = std::fs::read_dir(folder) {
        for entry in read.flatten() {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) && looks_like_skin(&path) && !found.contains(&path) {
                found.push(path);
            }
        }
    }
    found.sort_by_key(|path| skin_name(path).to_lowercase());
    found
}

pub fn own_skin_picture(side: u32) -> Vec<u8> {
    let mut made = image::RgbaImage::new(side, side);
    let own = image::DynamicImage::ImageRgba8(drawn_circle(side * 2, Some([255, 192, 0]), 0.22));
    laid(&mut made, &own, (side as f32 * 0.84) as u32);
    made.into_raw()
}

impl Settings {
    pub fn render_size(&self) -> (u32, u32) {
        let height = if HEIGHTS.contains(&self.render_height) { self.render_height } else { 1080 };
        ((height * 16 / 9 + 1) & !1, height)
    }

    pub fn renders_dir(&self) -> PathBuf {
        self.renders_dir.clone().unwrap_or_else(|| crate::sources::own_root().join("Renders"))
    }
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            lang: Lang::from_system(),
            sources: Vec::new(),
            device: device_name(),
            server: DEFAULT_SERVER.to_owned(),
            token: String::new(),
            linked_as: String::new(),
            menu_tab: String::new(),
            live_scene: true,
            pause_unfocused: true,
            render_height: 1080,
            render_fps: 60,
            render_crf: 20,
            renders_dir: None,
            settings_tab: String::new(),
            tiles_app: Vec::new(),
            tiles_bot: Vec::new(),
            chat_id: None,
            tell: Tell::default(),
            skin: None,
            own_skins: Vec::new(),
            music_level: 1.0,
            hitsound_level: 1.0,
            player_level: 1.0,
            player_muted: false,
            player_rate: 1.0,
            player_loop: false,
            hud: true,
            cursor_grows: false,
            background_dim: 0.82,
            background_blur: 1.0,
            map_sounds: true,
            skin_sounds: true,
        }
    }
}

pub fn path() -> PathBuf {
    crate::sources::home().join(".dossier").join("dossier.json")
}

pub fn first_run() -> bool {
    !path().is_file()
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let file = path();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&file, text).map_err(|e| e.to_string())
    }
}

pub fn device_name() -> String {
    let said = std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_default();
    let bare = said.split('.').next().unwrap_or("").trim();
    if bare.is_empty() {
        "Dossier".to_owned()
    } else {
        bare.replace('-', " ")
    }
}

#[cfg(test)]
mod tests {

    fn packed(files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut made = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut made));
            let plain = zip::write::SimpleFileOptions::default();
            for (name, bytes) in files {
                zip.start_file(*name, plain).unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        made
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-skins-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_osk_is_unpacked_under_its_own_name() {
        let root = scratch("plain");
        let file = root.join("Seoul v9.osk");
        std::fs::write(&file, packed(&[("skin.ini", b"[General]"), ("hitcircle.png", b"x")])).unwrap();
        let made = unpack_skin_into(&file, &root).unwrap();
        assert_eq!(made, root.join("Seoul v9"));
        assert!(looks_like_skin(&made));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_osk_wrapped_in_one_folder_is_lifted_out_of_it() {
        let root = scratch("wrapped");
        let file = root.join("Aristia.osk");
        std::fs::write(&file, packed(&[("Aristia/skin.ini", b"[General]"), ("Aristia/cursor.png", b"x")])).unwrap();
        let made = unpack_skin_into(&file, &root).unwrap();
        assert!(made.join("skin.ini").is_file(), "the skin's own files sit at the top");
        assert!(!made.join("Aristia").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_skin_file_left_in_the_folder_is_taken_in_once() {
        let root = scratch("adopt");
        std::fs::write(root.join("Rafis.osk"), packed(&[("skin.ini", b"[General]")])).unwrap();
        assert_eq!(adopt_skin_files(&root), vec![root.join("Rafis")]);
        std::fs::write(root.join("Rafis").join("marker"), b"mine").unwrap();
        assert_eq!(adopt_skin_files(&root), vec![root.join("Rafis")]);
        assert!(root.join("Rafis").join("marker").is_file(), "an unpacked skin is left as it is");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn everything_in_the_archive_lands_on_disk() {
        let root = scratch("whole");
        let file = root.join("Whole.osk");
        std::fs::write(
            &file,
            packed(&[
                ("skin.ini", b"[General]\nName: whole\n"),
                ("hitcircle.png", b"a"),
                ("hitcircleoverlay@2x.png", b"b"),
                ("sounds/normal-hitnormal.wav", b"c"),
                ("numbers/default-1.png", b"d"),
            ]),
        )
        .unwrap();
        let made = unpack_skin_into(&file, &root).unwrap();
        for at in ["skin.ini", "hitcircle.png", "hitcircleoverlay@2x.png", "sounds/normal-hitnormal.wav", "numbers/default-1.png"] {
            assert!(made.join(at).is_file(), "{at} came out of the archive");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_is_a_skin_when_it_holds_one_of_the_marks() {
        let root = scratch("marks");
        assert!(!looks_like_skin(&root));
        std::fs::write(root.join("hitcircle@2x.png"), b"x").unwrap();
        assert!(!looks_like_skin(&root), "one stray mark is not a skin");
        std::fs::write(root.join("cursor.png"), b"x").unwrap();
        std::fs::write(root.join("hit300.png"), b"x").unwrap();
        assert!(looks_like_skin(&root), "three of the marks are");
        std::fs::write(root.join("a map.osu"), b"x").unwrap();
        assert!(!looks_like_skin(&root), "a folder with a map in it is a mapset");
        assert!(is_skin_file(Path::new("a/b/Seoul.OSK")));
        assert_eq!(skin_name(Path::new("a/b/Seoul.osk")), "Seoul");
        assert_eq!(skin_name(Path::new("a/b/Seoul")), "Seoul");
        let _ = std::fs::remove_dir_all(&root);
    }
    use super::*;

    #[test]
    fn settings_survive_a_round_trip_through_json() {
        let mut said = Settings::default();
        said.lang = Lang::Ru;
        said.device = "MacBook Pro".to_owned();
        said.token = "abc".to_owned();
        let text = serde_json::to_string(&said).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(said, back);
    }

    #[test]
    fn a_device_always_has_a_name() {
        assert!(!device_name().is_empty());
    }
}
