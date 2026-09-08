use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use tiny_skia::{Color, Pixmap, PremultipliedColorU8};

use crate::elements::Element;

#[derive(Debug, Clone)]
pub struct Ini {
    pub hit_circle_overlap: f32,

    pub cursor_rotate: bool,

    pub score_overlap: f32,

    pub combo_overlap: f32,

    pub hit_circle_prefix: String,
    pub score_prefix: String,
    pub combo_prefix: String,

    pub version: f32,

    pub layered_hit_sounds: bool,

    pub overlay_above_number: bool,

    pub combo_colours: Vec<Color>,

    pub animation_framerate: f32,

    pub cursor_expand: bool,

    pub slider_ball_tint: bool,

    pub slider_ball_flip: bool,
    pub slider_border: Option<Color>,

    pub slider_track: Option<Color>,

    pub spinner_background: Option<Color>,

    pub input_overlay_text: Option<Color>,

    pub spinner_no_blink: bool,
}

impl Default for Ini {
    fn default() -> Self {
        Self {
            hit_circle_overlap: -2.0,
            cursor_rotate: true,
            score_overlap: 0.0,
            combo_overlap: 0.0,

            version: LATEST_SKIN_VERSION,
            layered_hit_sounds: true,
            overlay_above_number: true,
            hit_circle_prefix: "default".to_owned(),
            score_prefix: "score".to_owned(),
            combo_prefix: "score".to_owned(),
            combo_colours: Vec::new(),
            animation_framerate: -1.0,
            cursor_expand: true,
            slider_ball_tint: false,
            slider_ball_flip: false,
            slider_border: None,
            slider_track: None,
            spinner_background: None,
            input_overlay_text: None,
            spinner_no_blink: false,
        }
    }
}

impl Ini {
    pub fn read(root: &Path) -> Self {
        let index = index_of(root);
        let Some(path) = index.get("skin.ini") else {
            return Self::default();
        };
        fs::read(path)
            .ok()
            .map(|bytes| Self::parse(&String::from_utf8_lossy(&bytes)))
            .unwrap_or_default()
    }

    pub fn parse(text: &str) -> Self {
        let mut out = Self {
            version: 1.0,
            ..Self::default()
        };
        let mut numbered: Vec<(usize, Color)> = Vec::new();
        let mut section = String::new();

        for line in text.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.to_ascii_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());

            match (section.as_str(), key.as_str()) {
                ("fonts", "hitcircleoverlap") => {
                    if let Ok(n) = value.parse::<f32>() {
                        out.hit_circle_overlap = n;
                    }
                }
                ("fonts", "scoreoverlap") => {
                    if let Ok(n) = value.parse::<f32>() {
                        out.score_overlap = n;
                    }
                }
                ("fonts", "combooverlap") => {
                    if let Ok(n) = value.parse::<f32>() {
                        out.combo_overlap = n;
                    }
                }

                ("general", "version") => {
                    out.version = if value.eq_ignore_ascii_case("latest") {
                        LATEST_SKIN_VERSION
                    } else {
                        value.parse().unwrap_or(LATEST_SKIN_VERSION)
                    };
                }
                ("fonts", "hitcircleprefix") => out.hit_circle_prefix = named(value),
                ("fonts", "scoreprefix") => out.score_prefix = named(value),
                ("fonts", "comboprefix") => out.combo_prefix = named(value),

                ("general", "hitcircleoverlayabovenumber")
                | ("general", "hitcircleoverlayabovenumer") => {
                    out.overlay_above_number = value != "0";
                }
                ("general", "layeredhitsounds") => {
                    out.layered_hit_sounds = value != "0";
                }
                ("general", "cursorexpand") => out.cursor_expand = value != "0",
                ("general", "cursorrotate") => out.cursor_rotate = value != "0",
                ("general", "allowsliderballtint") => out.slider_ball_tint = value == "1",
                ("general", "sliderballflip") => out.slider_ball_flip = value == "1",
                ("general", "animationframerate") => {
                    if let Ok(n) = value.parse::<f32>() {
                        out.animation_framerate = n;
                    }
                }
                ("colours", "sliderborder") => out.slider_border = rgb_of(value),
                ("colours", "spinnerbackground") => out.spinner_background = rgb_of(value),
                ("colours", "inputoverlaytext") => out.input_overlay_text = rgb_of(value),
                ("general", "spinnernoblink") => out.spinner_no_blink = value == "1",
                ("colours", "slidertrackoverride") => out.slider_track = rgb_of(value),
                ("colours", _) if key.starts_with("combo") => {
                    if let (Ok(n), Some(colour)) = (
                        key.trim_start_matches("combo").parse::<usize>(),
                        rgb_of(value),
                    ) {
                        numbered.push((n, colour));
                    }
                }
                _ => {}
            }
        }

        numbered.sort_by_key(|(n, _)| *n);
        if numbered.len() > 1 {
            numbered.rotate_left(1);
        }
        out.combo_colours = numbered.into_iter().map(|(_, c)| c).collect();
        out
    }
}

fn rgb_of(value: &str) -> Option<Color> {
    let mut parts = value.split(',').map(|p| p.trim().parse::<u8>());
    let (r, g, b) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    Some(Color::from_rgba8(r, g, b, 255))
}

pub fn tinted(pixmap: &Pixmap, tint: Color) -> Pixmap {
    let mut out = pixmap.clone();
    let (r, g, b) = (tint.red(), tint.green(), tint.blue());
    for pixel in out.pixels_mut() {
        let (pr, pg, pb, pa) = (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha());
        *pixel = PremultipliedColorU8::from_rgba(
            (f32::from(pr) * r) as u8,
            (f32::from(pg) * g) as u8,
            (f32::from(pb) * b) as u8,
            pa,
        )
        .unwrap_or(*pixel);
    }
    out
}

#[derive(Clone)]
pub struct Sprite {
    pub pixmap: Pixmap,

    pub scale: f32,

    pub ink_width: f32,
    pub ink_height: f32,
}

impl Sprite {
    fn new(pixmap: Pixmap, scale: f32) -> Self {
        let (mut left, mut right) = (pixmap.width(), 0u32);
        let (mut top, mut bottom) = (pixmap.height(), 0u32);
        for (index, pixel) in pixmap.pixels().iter().enumerate() {
            if pixel.alpha() == 0 {
                continue;
            }
            let (x, y) = (index as u32 % pixmap.width(), index as u32 / pixmap.width());
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
        }
        let span = |from: u32, to: u32| {
            if from > to {
                0.0
            } else {
                (to - from + 1) as f32 / scale
            }
        };
        Self {
            ink_width: span(left, right),
            ink_height: span(top, bottom),
            pixmap,
            scale,
        }
    }

    pub fn width(&self) -> f32 {
        self.pixmap.width() as f32 / self.scale
    }

    pub fn height(&self) -> f32 {
        self.pixmap.height() as f32 / self.scale
    }

    fn is_blank(&self) -> bool {
        self.pixmap.pixels().iter().all(|p| p.alpha() == 0)
    }

    fn tinted(&self, tint: Color) -> Pixmap {
        tinted(&self.pixmap, tint)
    }
}

pub struct Sprites {
    have: HashMap<Element, Sprite>,

    frames: HashMap<Element, Vec<Sprite>>,

    off: HashSet<Element>,

    ini: Ini,

    palette: usize,

    tinted: HashMap<(Element, usize), Pixmap>,
}

impl std::fmt::Debug for Sprites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sprites")
            .field("pictures", &self.have.len())
            .field("animated", &self.frames.len())
            .field("turned off", &self.off.len())
            .field("tinted copies", &self.tinted.len())
            .finish()
    }
}

impl Sprites {
    pub fn read(root: &Path, wanted: &[Element]) -> Self {
        let index = index_of(root);

        let ini = index
            .get("skin.ini")
            .and_then(|p| fs::read(p).ok())
            .map(|b| Ini::parse(&String::from_utf8_lossy(&b)))
            .unwrap_or_default();
        let mut have = HashMap::new();
        let mut frames: HashMap<Element, Vec<Sprite>> = HashMap::new();
        let mut off = HashSet::new();

        for &element in wanted {
            let named = element.stem_with(&ini).to_ascii_lowercase();
            let stem = if named.contains('/') && !holds(&index, &named) {
                leaf_of(&named)
            } else {
                named
            };

            let found = index
                .get(&format!("{stem}-0@2x.png"))
                .map(|p| (p, 2.0))
                .or_else(|| index.get(&format!("{stem}-0.png")).map(|p| (p, 1.0)))
                .or_else(|| index.get(&format!("{stem}0@2x.png")).map(|p| (p, 2.0)))
                .or_else(|| index.get(&format!("{stem}0.png")).map(|p| (p, 1.0)))
                .or_else(|| index.get(&format!("{stem}@2x.png")).map(|p| (p, 2.0)))
                .or_else(|| index.get(&format!("{stem}.png")).map(|p| (p, 1.0)));
            let Some((path, scale)) = found else { continue };
            let Some(pixmap) = fs::read(path)
                .ok()
                .and_then(|b| Pixmap::decode_png(&b).ok())
            else {
                continue;
            };
            let first = Sprite::new(pixmap, scale);

            let mut strip = vec![first];
            for n in 1.. {
                let next = index
                    .get(&format!("{stem}-{n}@2x.png"))
                    .map(|p| (p, 2.0))
                    .or_else(|| index.get(&format!("{stem}-{n}.png")).map(|p| (p, 1.0)))
                    .or_else(|| index.get(&format!("{stem}{n}@2x.png")).map(|p| (p, 2.0)))
                    .or_else(|| index.get(&format!("{stem}{n}.png")).map(|p| (p, 1.0)));
                let Some((path, scale)) = next else { break };
                match fs::read(path)
                    .ok()
                    .and_then(|b| Pixmap::decode_png(&b).ok())
                {
                    Some(pixmap) => strip.push(Sprite::new(pixmap, scale)),
                    None => break,
                }
            }

            if strip.iter().all(Sprite::is_blank) {
                off.insert(element);
                continue;
            }

            let resting = strip.iter().position(|s| !s.is_blank()).unwrap_or(0);
            if strip.len() > 1 {
                frames.insert(element, strip.clone());
            }
            have.insert(element, strip.swap_remove(resting));
        }
        Self {
            have,
            frames,
            off,
            ini,
            palette: 0,
            tinted: HashMap::new(),
        }
    }

    pub fn frame_count(&self, element: Element) -> usize {
        self.frames.get(&element).map_or(1, Vec::len)
    }

    pub fn frame(&self, element: Element, frame: usize) -> Option<(&Pixmap, f32)> {
        let strip = self.frames.get(&element)?;
        let sprite = strip.get(frame % strip.len())?;
        Some((&sprite.pixmap, sprite.scale))
    }

    pub fn ini(&self) -> &Ini {
        &self.ini
    }

    pub fn allow_slider_ball_tint(&mut self, yes: bool) {
        self.ini.slider_ball_tint = yes;
    }

    pub fn tint_for(mut self, colours: &[Color]) -> Self {
        self.palette = colours.len();
        for (&element, sprite) in &self.have {
            if !element.is_tinted() {
                continue;
            }

            if element == Element::SliderBall && !self.ini.slider_ball_tint {
                continue;
            }
            for (index, &colour) in colours.iter().enumerate() {
                self.tinted.insert((element, index), sprite.tinted(colour));
            }
        }
        self
    }

    pub fn coloured(&self, element: Element, combo: usize) -> Option<(&Pixmap, f32)> {
        let scale = self.have.get(&element)?.scale;
        if element.is_tinted() {
            if self.palette == 0 {
                return None;
            }
            if let Some(painted) = self.tinted.get(&(element, combo % self.palette)) {
                return Some((painted, scale));
            }
        }
        self.have.get(&element).map(|s| (&s.pixmap, scale))
    }

    pub fn get(&self, element: Element) -> Option<&Sprite> {
        self.have.get(&element)
    }

    pub fn silenced(&self, element: Element) -> bool {
        self.off.contains(&element)
    }

    pub fn draw_ourselves(&self, element: Element) -> bool {
        !self.have.contains_key(&element) && !self.off.contains(&element)
    }

    pub fn len(&self) -> usize {
        self.have.len()
    }

    pub fn is_empty(&self) -> bool {
        self.have.is_empty()
    }
}

pub const LATEST_SKIN_VERSION: f32 = 2.7;

pub fn effective_version(stated: f32, as_written: bool) -> f32 {
    if as_written {
        stated
    } else {
        stated.max(LATEST_SKIN_VERSION)
    }
}

fn named(value: &str) -> String {
    value.trim().replace('\\', "/")
}

fn leaf_of(value: &str) -> String {
    value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .trim()
        .to_owned()
}

fn holds(index: &HashMap<String, PathBuf>, stem: &str) -> bool {
    ["-0@2x", "-0", "0@2x", "0", "@2x", ""]
        .iter()
        .any(|tail| index.contains_key(&format!("{stem}{tail}.png")))
}

fn index_of(root: &Path) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();
    walk(root, "", &mut index);
    index
}

fn walk(dir: &Path, under: &str, index: &mut HashMap<String, PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if under.is_empty() {
                walk(&entry.path(), &format!("{name}/"), index);
            }
            continue;
        }
        if !kind.is_file() {
            continue;
        }
        index.insert(format!("{under}{name}"), entry.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::Verdict;

    #[test]
    fn a_prefix_that_names_a_subfolder_is_found() {
        let dir = std::env::temp_dir().join(format!("dossier-skin-nested-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("num")).expect("a skin folder");
        fs::write(dir.join("skin.ini"), "[Fonts]\nScorePrefix: num\\berlin\n").expect("an ini");
        write(&dir.join("num"), "berlin-0.png", 32, 255);

        let index = index_of(&dir);
        assert!(
            index.contains_key("num/berlin-0.png"),
            "a nested file was not indexed: {:?}",
            index.keys().collect::<Vec<_>>()
        );

        assert!(!index.contains_key("berlin-0.png"));

        let sprites = Sprites::read(&dir, &[Element::Score('0')]);
        assert!(
            sprites.get(Element::Score('0')).is_some(),
            "the skin's own digit was not picked up"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_overlay_key_is_honoured_under_the_spelling_skins_actually_use() {
        assert!(!Ini::parse("[General]\nHitCircleOverlayAboveNumer: 0\n").overlay_above_number);
        assert!(Ini::parse("[General]\nHitCircleOverlayAboveNumer: 1\n").overlay_above_number);

        assert!(!Ini::parse("[General]\nHitCircleOverlayAboveNumber: 0\n").overlay_above_number);

        assert!(Ini::parse("[General]\n").overlay_above_number);
    }

    #[test]
    fn a_skin_can_turn_off_the_hit_under_its_decorations() {
        assert!(Ini::default().layered_hit_sounds);
        assert!(Ini::parse("[General]\nName: x").layered_hit_sounds);
        assert!(!Ini::parse("[General]\nLayeredHitSounds: 0").layered_hit_sounds);
        assert!(Ini::parse("[General]\nLayeredHitSounds: 1").layered_hit_sounds);
    }

    #[test]
    fn a_skin_ini_dates_the_skin_and_its_absence_does_not() {
        assert_eq!(
            Ini::default().version,
            LATEST_SKIN_VERSION,
            "no file at all"
        );
        assert_eq!(
            Ini::parse("[General]\nName: something").version,
            1.0,
            "a file that says nothing about its version is a version 1 skin"
        );
        assert_eq!(Ini::parse("[General]\nVersion: 2.5").version, 2.5);

        assert_eq!(
            Ini::parse("[General]\nVersion: latest").version,
            LATEST_SKIN_VERSION
        );

        assert_eq!(
            Ini::parse("[General]\nVersion: ?").version,
            LATEST_SKIN_VERSION
        );
    }

    fn write(dir: &Path, name: &str, size: u32, alpha: u8) {
        let mut pixmap = Pixmap::new(size, size).expect("a canvas");
        for pixel in pixmap.pixels_mut() {
            *pixel = tiny_skia::PremultipliedColorU8::from_rgba(alpha, alpha, alpha, alpha)
                .expect("a colour");
        }
        fs::write(dir.join(name), pixmap.encode_png().expect("png")).expect("written");
    }

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-skin-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    const WANTED: &[Element] = &[
        Element::HitCircle,
        Element::ApproachCircle,
        Element::CursorTrail,
        Element::Verdict(Verdict::Three),
    ];

    #[test]
    fn a_skin_that_has_the_picture_hands_it_over() {
        let dir = folder("plain");
        write(&dir, "hitcircle.png", 128, 255);
        let sprites = Sprites::read(&dir, WANTED);

        let sprite = sprites.get(Element::HitCircle).expect("it was there");
        assert_eq!(sprite.scale, 1.0);
        assert_eq!(sprite.width(), 128.0);
        assert!(!sprites.draw_ourselves(Element::HitCircle));
    }

    #[test]
    fn the_high_resolution_file_wins_and_says_what_its_size_means() {
        let dir = folder("at2x");
        write(&dir, "hitcircle.png", 128, 255);
        write(&dir, "hitcircle@2x.png", 256, 255);
        let sprites = Sprites::read(&dir, WANTED);

        let sprite = sprites.get(Element::HitCircle).expect("it was there");
        assert_eq!(sprite.scale, 2.0);
        assert_eq!(sprite.pixmap.width(), 256);
        assert_eq!(sprite.width(), 128.0, "the same element, at the same size");
    }

    #[test]
    fn an_animations_first_frame_is_the_element() {
        let dir = folder("anim");
        write(&dir, "hitcircle-0.png", 128, 255);
        let sprites = Sprites::read(&dir, WANTED);
        assert!(sprites.get(Element::HitCircle).is_some());
    }

    #[test]
    fn the_slider_balls_frames_carry_no_dash() {
        let dir = folder("sliderb");
        write(&dir, "sliderb0@2x.png", 256, 255);
        let sprites = Sprites::read(&dir, &[Element::SliderBall]);
        let ball = sprites.get(Element::SliderBall).expect("found");
        assert_eq!(ball.scale, 2.0, "the suffix still counts");
    }

    #[test]
    fn a_blank_first_frame_turns_the_element_off_like_any_other_blank() {
        let dir = folder("anim-blank");
        write(&dir, "hitcircle-0.png", 1, 0);
        let sprites = Sprites::read(&dir, WANTED);
        assert!(sprites.silenced(Element::HitCircle));
        assert!(!sprites.draw_ourselves(Element::HitCircle));
    }

    #[test]
    fn an_animation_is_preferred_to_the_still_beside_it() {
        let dir = folder("anim-both");
        write(&dir, "hitcircle.png", 128, 255);
        write(&dir, "hitcircle-0.png", 1, 0);
        let sprites = Sprites::read(&dir, WANTED);
        assert!(
            sprites.silenced(Element::HitCircle),
            "the numbered frame is the one osu! reads"
        );
    }

    #[test]
    fn a_skin_made_on_windows_still_loads() {
        let dir = folder("case");
        write(&dir, "HitCircle.png", 128, 255);
        write(&dir, "ApproachCircle@2X.png", 252, 255);
        let sprites = Sprites::read(&dir, WANTED);

        assert!(sprites.get(Element::HitCircle).is_some());
        let approach = sprites.get(Element::ApproachCircle).expect("found");
        assert_eq!(approach.scale, 2.0, "the suffix counts whatever its case");
    }

    #[test]
    fn a_blank_picture_means_off_and_a_missing_one_means_default() {
        let dir = folder("blank");
        write(&dir, "cursortrail.png", 1, 0);
        let sprites = Sprites::read(&dir, WANTED);

        assert!(sprites.silenced(Element::CursorTrail));
        assert!(sprites.get(Element::CursorTrail).is_none());
        assert!(
            !sprites.draw_ourselves(Element::CursorTrail),
            "it was turned off, not left out"
        );

        let missing = Element::Verdict(Verdict::Three);
        assert!(!sprites.silenced(missing));
        assert!(
            sprites.draw_ourselves(missing),
            "nothing was said about it, so it is ours to draw"
        );
    }

    #[test]
    fn a_picture_with_colour_under_a_zero_alpha_is_still_off() {
        let dir = folder("ghost");
        let mut pixmap = Pixmap::new(128, 128).expect("a canvas");
        for pixel in pixmap.pixels_mut() {
            *pixel = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).expect("a colour");
        }
        fs::write(dir.join("hitcircle.png"), pixmap.encode_png().expect("png")).expect("written");

        let sprites = Sprites::read(&dir, WANTED);
        assert!(sprites.silenced(Element::HitCircle));
        assert!(!sprites.draw_ourselves(Element::HitCircle));
    }

    #[test]
    fn nothing_below_the_top_of_the_folder_is_read() {
        let dir = folder("nested");
        let nested = dir.join("cursors");
        fs::create_dir_all(&nested).expect("a folder");
        write(&nested, "hitcircle.png", 128, 255);
        let sprites = Sprites::read(&dir, WANTED);

        assert!(sprites.get(Element::HitCircle).is_none());
        assert!(sprites.draw_ourselves(Element::HitCircle));
    }

    #[test]
    fn a_tinted_element_comes_back_wearing_the_combo_colour() {
        let dir = folder("tint");
        write(&dir, "hitcircle.png", 8, 255);
        let sprites = Sprites::read(&dir, WANTED).tint_for(&[
            Color::from_rgba8(255, 0, 0, 255),
            Color::from_rgba8(0, 0, 255, 255),
        ]);

        let (first, _) = sprites.coloured(Element::HitCircle, 0).expect("combo 0");
        let (second, _) = sprites.coloured(Element::HitCircle, 1).expect("combo 1");
        let px = |p: &Pixmap| {
            let q = p.pixels()[0];
            (q.red(), q.green(), q.blue())
        };
        assert_eq!(px(first).0, 255, "the first combo is red");
        assert_eq!(px(first).2, 0);
        assert_eq!(px(second).2, 255, "the second is blue");
        assert_eq!(px(second).0, 0);
    }

    #[test]
    fn a_combo_past_the_end_of_the_palette_wraps_round_it() {
        let dir = folder("wrap");
        write(&dir, "hitcircle.png", 8, 255);
        let sprites = Sprites::read(&dir, WANTED).tint_for(&[
            Color::from_rgba8(255, 0, 0, 255),
            Color::from_rgba8(0, 255, 0, 255),
        ]);

        let first = sprites.coloured(Element::HitCircle, 0).expect("combo 0");
        for combo in [2usize, 4, 100] {
            let later = sprites
                .coloured(Element::HitCircle, combo)
                .unwrap_or_else(|| panic!("combo {combo} drew nothing"));
            assert_eq!(later.0.pixels()[0], first.0.pixels()[0], "combo {combo}");
        }
        assert!(
            sprites.coloured(Element::HitCircle, 3).is_some(),
            "odd combos too"
        );
    }

    #[test]
    fn a_blanked_hud_glyph_reads_as_off_rather_than_as_missing() {
        let dir = folder("hud-blank");
        write(&dir, "score-4.png", 8, 255);
        write(&dir, "score-x.png", 8, 0);
        let wanted = [
            Element::Score('4'),
            Element::Score('x'),
            Element::Score('7'),
        ];
        let sprites = Sprites::read(&dir, &wanted);

        assert!(
            sprites.get(Element::Score('4')).is_some(),
            "a figure it drew"
        );
        assert!(sprites.silenced(Element::Score('x')), "a sign it deleted");
        assert!(
            sprites.draw_ourselves(Element::Score('7')),
            "and one it never mentioned, which is the only case worth a fallback"
        );
    }

    #[test]
    fn a_skin_that_was_never_tinted_draws_nothing_tinted() {
        let dir = folder("untinted-skin");
        write(&dir, "hitcircle.png", 8, 255);
        let sprites = Sprites::read(&dir, WANTED);
        assert!(sprites.coloured(Element::HitCircle, 0).is_none());
    }

    #[test]
    fn an_untinted_element_is_handed_over_as_the_skin_drew_it() {
        let dir = folder("untinted");
        write(&dir, "reversearrow.png", 8, 255);
        let sprites = Sprites::read(&dir, &[Element::ReverseArrow])
            .tint_for(&[Color::from_rgba8(0, 255, 0, 255)]);

        let (drawn, _) = sprites.coloured(Element::ReverseArrow, 0).expect("there");
        let original = sprites.get(Element::ReverseArrow).expect("there");
        assert_eq!(drawn.pixels()[0], original.pixmap.pixels()[0]);
    }

    #[test]
    fn tinting_keeps_the_shape_the_skin_drew() {
        let dir = folder("alpha");
        let mut pixmap = Pixmap::new(2, 1).expect("a canvas");
        pixmap.pixels_mut()[0] =
            PremultipliedColorU8::from_rgba(128, 128, 128, 128).expect("half-lit");
        pixmap.pixels_mut()[1] = PremultipliedColorU8::from_rgba(0, 0, 0, 0).expect("clear");
        fs::write(dir.join("hitcircle.png"), pixmap.encode_png().expect("png")).expect("written");

        let sprites =
            Sprites::read(&dir, WANTED).tint_for(&[Color::from_rgba8(255, 255, 255, 255)]);
        let (out, _) = sprites.coloured(Element::HitCircle, 0).expect("there");
        assert_eq!(out.pixels()[0].alpha(), 128, "the soft edge stays soft");
        assert_eq!(out.pixels()[1].alpha(), 0, "and the clear part stays clear");
    }

    #[test]
    fn the_ini_is_read_for_the_settings_that_change_a_play() {
        let ini = Ini::parse(
            "[General]\nName: doki dt mix v3\n\n[Colours]\nCombo1: 255,255,255\n             Combo2: 10,20,30\n\n[Fonts]\nHitCirclePrefix: default\nHitCircleOverlap: 160\n",
        );
        assert_eq!(ini.hit_circle_overlap, 160.0);

        assert_eq!(ini.combo_colours.len(), 2);
        assert_eq!(ini.combo_colours[0], Color::from_rgba8(10, 20, 30, 255));
        assert_eq!(ini.combo_colours[1], Color::from_rgba8(255, 255, 255, 255));
    }

    #[test]
    fn an_absurd_overlap_is_taken_at_its_word() {
        assert_eq!(
            Ini::parse("[Fonts]\nHitCircleOverlap: 160\n").hit_circle_overlap,
            160.0
        );
    }

    #[test]
    fn the_mania_section_does_not_repaint_the_notes() {
        let ini = Ini::parse("[Mania]\nColour1: 255,0,0\nCombo1: 0,255,0\n");
        assert!(ini.combo_colours.is_empty(), "{:?}", ini.combo_colours);
    }

    #[test]
    fn comments_and_a_missing_ini_are_both_ordinary() {
        let ini = Ini::parse("// a note to nobody\n[Fonts]\nHitCircleOverlap: 7 // why not\n");
        assert_eq!(ini.hit_circle_overlap, 7.0);

        assert_eq!(
            Ini::read(Path::new("/no/such/skin")).hit_circle_overlap,
            -2.0
        );
    }

    #[test]
    fn a_skin_that_renames_its_digits_is_still_found() {
        let ini = Ini::parse("[Fonts]\nHitCirclePrefix: numbers/hit\nScorePrefix: ui\\score\n");

        assert_eq!(Element::Digit(4).stem_with(&ini), "numbers/hit-4");
        assert_eq!(Element::Score('x').stem_with(&ini), "ui/score-x");

        let plain = Ini::default();
        assert_eq!(Element::Digit(4).stem_with(&plain), "default-4");
        assert_eq!(Element::Score(',').stem_with(&plain), "score-comma");
        assert_eq!(plain.combo_prefix, "score");
    }

    #[test]
    fn a_folder_that_is_not_there_is_not_a_failure() {
        let sprites = Sprites::read(Path::new("/no/such/skin"), WANTED);
        assert!(sprites.is_empty());
        assert!(WANTED.iter().all(|&e| sprites.draw_ourselves(e)));
    }

    #[test]
    fn a_file_that_is_not_a_png_is_skipped_rather_than_fatal() {
        let dir = folder("junk");
        fs::write(dir.join("hitcircle.png"), b"not a png at all").expect("written");
        let sprites = Sprites::read(&dir, WANTED);
        assert!(sprites.draw_ourselves(Element::HitCircle));
    }

    #[test]
    fn a_blank_first_frame_is_not_a_skin_turning_the_element_off() {
        let dir = folder("animated");
        write(&dir, "followpoint-0.png", 32, 0);
        write(&dir, "followpoint-1.png", 32, 200);
        write(&dir, "followpoint-2.png", 32, 255);
        let sprites = Sprites::read(&dir, &[Element::FollowPoint]);

        assert!(
            !sprites.draw_ourselves(Element::FollowPoint),
            "it is skinned"
        );
        assert_eq!(sprites.frame_count(Element::FollowPoint), 3);
        assert!(
            sprites
                .get(Element::FollowPoint)
                .is_some_and(|s| !s.is_blank()),
            "and the still picture is a frame with something in it"
        );
    }

    #[test]
    fn a_strip_of_nothing_but_blanks_is_still_a_skin_saying_no() {
        let dir = folder("animated-off");
        write(&dir, "followpoint-0.png", 32, 0);
        write(&dir, "followpoint-1.png", 32, 0);
        let sprites = Sprites::read(&dir, &[Element::FollowPoint]);

        assert!(
            !sprites.draw_ourselves(Element::FollowPoint),
            "still spoken for"
        );
        assert!(
            sprites.get(Element::FollowPoint).is_none(),
            "and drawn as nothing"
        );
    }

    #[test]
    fn a_single_picture_has_one_frame() {
        let dir = folder("still");
        write(&dir, "hitcircle.png", 128, 255);
        let sprites = Sprites::read(&dir, &[Element::HitCircle]);
        assert_eq!(sprites.frame_count(Element::HitCircle), 1);
        assert!(
            sprites.frame(Element::HitCircle, 0).is_none(),
            "no strip to index"
        );
    }

    #[test]
    fn the_rate_a_strip_plays_at_defaults_to_the_whole_thing_in_a_second() {
        assert_eq!(Ini::default().animation_framerate, -1.0);
        assert_eq!(
            Ini::parse("[General]\nAnimationFramerate: 24\n").animation_framerate,
            24.0
        );
    }

    #[test]
    fn a_skin_may_ask_for_its_cursor_not_to_swell() {
        assert!(
            Ini::default().cursor_expand,
            "on unless a skin says otherwise"
        );
        assert!(!Ini::parse("[General]\nCursorExpand: 0\n").cursor_expand);
        assert!(Ini::parse("[General]\nCursorExpand: 1\n").cursor_expand);
    }

    #[test]
    fn the_slider_ball_keeps_its_own_colours_unless_asked() {
        assert!(
            !Ini::default().slider_ball_tint,
            "off unless a skin says otherwise"
        );
        assert!(Ini::parse("[General]\nAllowSliderBallTint: 1\n").slider_ball_tint);
    }

    #[test]
    fn the_ball_is_only_mirrored_where_a_skin_said_so() {
        assert!(
            !Ini::default().slider_ball_flip,
            "the game's default is a ball that comes back the way it went"
        );
        assert!(Ini::parse("[General]\nSliderBallFlip: 1\n").slider_ball_flip);
        assert!(!Ini::parse("[General]\nSliderBallFlip: 0\n").slider_ball_flip);
    }

    #[test]
    fn a_ball_left_alone_is_still_drawn() {
        let dir = folder("ball");
        write(&dir, "sliderb0.png", 128, 255);
        let sprites = Sprites::read(&dir, &[Element::SliderBall])
            .tint_for(&[Color::from_rgba8(255, 0, 0, 255)]);
        assert!(
            sprites.coloured(Element::SliderBall, 0).is_some(),
            "the ball vanished instead of keeping its own colours"
        );
    }

    #[test]
    fn a_ball_a_skin_asks_to_tint_is_tinted() {
        let dir = folder("ball-tinted");
        write(&dir, "sliderb0.png", 128, 255);
        std::fs::write(dir.join("skin.ini"), "[General]\nAllowSliderBallTint: 1\n")
            .expect("written");
        let sprites = Sprites::read(&dir, &[Element::SliderBall])
            .tint_for(&[Color::from_rgba8(255, 0, 0, 255)]);
        let (art, _) = sprites.coloured(Element::SliderBall, 0).expect("drawn");
        let pixel = art.pixels()[0];
        assert!(pixel.red() > 0 && pixel.green() == 0, "{pixel:?}");
    }
}

#[cfg(test)]
mod drawn_by {
    use super::{effective_version, Ini, LATEST_SKIN_VERSION};

    #[test]
    fn a_skin_that_never_mentioned_a_version_is_drawn_by_the_newest_rules() {
        let stated = Ini::parse("[General]\nName: something").version;
        assert_eq!(stated, 1.0, "the file still says what it says");
        assert_eq!(effective_version(stated, false), LATEST_SKIN_VERSION);
    }

    #[test]
    fn a_skin_that_asked_for_the_newest_rules_is_left_alone() {
        assert_eq!(
            effective_version(LATEST_SKIN_VERSION, false),
            LATEST_SKIN_VERSION
        );
        assert_eq!(effective_version(3.5, false), 3.5, "never lowered");
    }

    #[test]
    fn the_clients_own_answer_is_one_flag_away() {
        assert_eq!(effective_version(1.0, true), 1.0);
    }
}
