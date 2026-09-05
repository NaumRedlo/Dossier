use dossier_beatmap::Colour;
use tiny_skia::Color;

use crate::imported::Sprites;
use crate::text::Font;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowShape {
    Triangle,

    Rounded,
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub sprites: Option<std::sync::Arc<Sprites>>,

    pub combo_colours: Vec<Color>,
    pub background: Color,
    pub circle_border: Color,
    pub approach_circle: Color,
    pub slider_border: Color,

    pub hit_lighting: bool,

    pub snake_in: bool,
    pub snake_out: bool,
    pub cursor_expand: bool,
    pub cursor_trail: bool,
    pub keypad: bool,
    pub key_bars: bool,
    pub unstable_rate: bool,
    pub slider_body: Option<Color>,

    pub slider_body_dim: f32,
    pub slider_body_alpha: f32,

    pub border_ratio: f32,

    pub arrow: ArrowShape,
    pub cursor: Color,
    pub trail_colour: Color,
    pub spinner: Color,

    pub font: Option<Font>,
    pub hud: Color,

    pub podium: [Color; 3],

    pub verdict_300: Color,
    pub verdict_100: Color,
    pub verdict_50: Color,
    pub verdict_miss: Color,

    pub note_relief: f32,

    pub arrow_beat: f32,

    pub note_glow: f32,

    pub background_dim: f32,
    pub background_blur: f32,

    pub cursor_scale: f32,
    pub meter_scale: f32,

    pub skin_version_as_written: bool,

    pub cursor_rotate: Option<bool>,

    pub slider_ball_tint: bool,

    pub show_300: bool,
}

impl Skin {
    pub fn with_combo_colours(colours: &[Colour]) -> Self {
        Self {
            combo_colours: colours.iter().map(|c| rgb(c.r, c.g, c.b)).collect(),
            ..Self::default()
        }
    }

    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn combo_colour(&self, index: usize) -> Color {
        if self.combo_colours.is_empty() {
            return rgb(255, 192, 0);
        }
        self.combo_colours[index % self.combo_colours.len()]
    }
}

impl Default for Skin {
    fn default() -> Self {
        Self {
            sprites: None,
            combo_colours: dossier_beatmap::DEFAULT_COMBO_COLOURS
                .iter()
                .map(|c| rgb(c.r, c.g, c.b))
                .collect(),
            background: rgb(12, 12, 16),
            circle_border: rgb(255, 255, 255),
            approach_circle: rgb(255, 255, 255),
            slider_border: rgb(255, 255, 255),
            hit_lighting: false,
            snake_in: false,
            snake_out: false,
            cursor_expand: false,
            cursor_trail: true,
            keypad: true,
            key_bars: true,
            unstable_rate: true,
            slider_body: None,
            slider_body_dim: 0.35,
            slider_body_alpha: 0.70,
            border_ratio: 0.11,
            arrow: ArrowShape::Triangle,
            cursor: rgb(255, 255, 255),
            trail_colour: rgb(255, 190, 190),
            spinner: rgb(190, 190, 200),
            font: None,
            hud: rgb(255, 255, 255),
            podium: [rgb(255, 215, 0), rgb(192, 192, 210), rgb(205, 150, 80)],
            verdict_300: rgb(102, 204, 255),
            verdict_100: rgb(136, 221, 68),
            verdict_50: rgb(255, 204, 34),
            verdict_miss: rgb(237, 84, 84),

            note_relief: 0.0,
            arrow_beat: 0.0,
            note_glow: 0.0,
            background_dim: 0.82,
            background_blur: 0.022,
            cursor_scale: 1.0,
            meter_scale: 1.0,
            skin_version_as_written: false,
            cursor_rotate: None,
            slider_ball_tint: false,
            show_300: true,
        }
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

pub fn with_alpha(colour: Color, alpha: f32) -> Color {
    let mut out = colour;
    out.set_alpha(colour.alpha() * alpha.clamp(0.0, 1.0));
    out
}

pub fn lighten(colour: Color, amount: f32) -> Color {
    let k = amount.clamp(0.0, 1.0);
    Color::from_rgba(
        colour.red() + (1.0 - colour.red()) * k,
        colour.green() + (1.0 - colour.green()) * k,
        colour.blue() + (1.0 - colour.blue()) * k,
        colour.alpha(),
    )
    .unwrap_or(colour)
}

pub fn blend(from: Color, to: Color, amount: f32) -> Color {
    let k = amount.clamp(0.0, 1.0);
    let mix = |a: f32, b: f32| a + (b - a) * k;
    Color::from_rgba(
        mix(from.red(), to.red()),
        mix(from.green(), to.green()),
        mix(from.blue(), to.blue()),
        mix(from.alpha(), to.alpha()),
    )
    .unwrap_or(from)
}

pub fn body_outer(track: Color) -> Color {
    let scale = 1.1;
    Color::from_rgba(
        track.red() / scale,
        track.green() / scale,
        track.blue() / scale,
        track.alpha(),
    )
    .unwrap_or(track)
}

pub fn body_inner(track: Color) -> Color {
    let (amount, scale) = (0.25f32, 1.125f32);
    let lift = |c: f32| (c * scale + amount).min(1.0);
    Color::from_rgba(
        lift(track.red()),
        lift(track.green()),
        lift(track.blue()),
        track.alpha(),
    )
    .unwrap_or(track)
}

pub fn darken(colour: Color, amount: f32) -> Color {
    let k = 1.0 - amount.clamp(0.0, 1.0);
    Color::from_rgba(
        colour.red() * k,
        colour.green() * k,
        colour.blue() * k,
        colour.alpha(),
    )
    .unwrap_or(colour)
}

#[cfg(test)]
mod shades {
    use super::*;

    #[test]
    fn lightening_moves_toward_white_without_losing_the_hue() {
        let coral = rgb(226, 72, 72);
        let pale = lighten(coral, 0.45);

        assert!(pale.red() > coral.red());
        assert!(pale.green() > coral.green());
        assert!(pale.blue() > coral.blue());
        assert_eq!(pale.alpha(), coral.alpha(), "opacity is not the lever here");

        assert!(pale.red() > pale.green() + 0.1, "{pale:?}");
    }

    #[test]
    fn the_two_ends_are_the_colour_itself_and_white() {
        let coral = rgb(226, 72, 72);
        assert_eq!(lighten(coral, 0.0), coral);
        let white = lighten(coral, 1.0);
        assert!(white.red() > 0.99 && white.green() > 0.99 && white.blue() > 0.99);
    }

    #[test]
    fn blending_travels_the_whole_way_and_stops_at_both_ends() {
        let a = Color::from_rgba8(20, 20, 24, 255);
        let b = Color::from_rgba8(226, 72, 72, 255);
        assert_eq!(blend(a, b, 0.0), a);
        assert_eq!(blend(a, b, 1.0), b);

        let half = blend(a, b, 0.5);
        assert!((half.red() - (a.red() + b.red()) / 2.0).abs() < 1e-6);
        assert!((half.green() - (a.green() + b.green()) / 2.0).abs() < 1e-6);

        assert_eq!(blend(a, b, -1.0), a);
        assert_eq!(blend(a, b, 2.0), b);
    }

    #[test]
    fn lightening_and_darkening_pull_opposite_ways() {
        let coral = rgb(226, 72, 72);
        assert!(lighten(coral, 0.5).green() > coral.green());
        assert!(darken(coral, 0.5).green() < coral.green());
    }
}

#[cfg(test)]
mod body_shades {
    use super::*;

    #[test]
    fn a_black_track_still_has_light_down_the_middle() {
        let black = Color::from_rgba8(0, 0, 0, 255);
        assert_eq!(body_outer(black).red(), 0.0);
        assert!(
            (body_inner(black).red() - 0.25).abs() < 1e-6,
            "{}",
            body_inner(black).red()
        );
    }

    #[test]
    fn the_rim_is_barely_darker_than_the_track() {
        let blue = Color::from_rgba8(100, 150, 250, 255);
        let rim = body_outer(blue);
        assert!((rim.red() - blue.red() / 1.1).abs() < 1e-6);
        assert!(rim.red() > blue.red() * 0.85, "only slightly darker");
    }

    #[test]
    fn the_centre_is_lifted_a_long_way_and_never_past_white() {
        let blue = Color::from_rgba8(100, 150, 250, 255);
        let core = body_inner(blue);
        assert!(core.green() > blue.green(), "lighter than the track");
        for channel in [core.red(), core.green(), core.blue()] {
            assert!((0.0..=1.0).contains(&channel), "{channel}");
        }

        let pale = Color::from_rgba8(250, 250, 250, 255);
        assert!(body_inner(pale).red() <= 1.0);
    }

    #[test]
    fn the_middle_is_always_lighter_than_the_rim() {
        for (r, g, b) in [(0, 0, 0), (255, 255, 255), (12, 200, 40), (200, 30, 90)] {
            let track = Color::from_rgba8(r, g, b, 255);
            let (rim, core) = (body_outer(track), body_inner(track));
            assert!(
                core.red() >= rim.red() && core.green() >= rim.green() && core.blue() >= rim.blue(),
                "{r},{g},{b}"
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effects;

impl Effects {
    pub const ALL: [&'static str; 9] = [
        "snake-in",
        "snake-out",
        "cursor-expand",
        "cursor-trail",
        "keypad",
        "key-bars",
        "unstable-rate",
        "hit-lighting",
        "slider-ball-tint",
    ];

    pub fn apply(skin: &mut Skin, list: &str) {
        let named: Vec<&str> = list
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect();
        let on = |name: &str| named.contains(&name);
        skin.snake_in = on("snake-in");
        skin.snake_out = on("snake-out");
        skin.cursor_expand = on("cursor-expand");
        skin.cursor_trail = on("cursor-trail");
        skin.keypad = on("keypad");
        skin.key_bars = on("key-bars");
        skin.unstable_rate = on("unstable-rate");
        skin.hit_lighting = on("hit-lighting");
        skin.slider_ball_tint = on("slider-ball-tint");
    }

    pub fn asked_for(list: &str, name: &str) -> bool {
        list.split(',').map(str::trim).any(|named| named == name)
    }

    pub fn of(skin: &Skin) -> Vec<&'static str> {
        let mut on = Vec::new();
        for (name, set) in [
            ("snake-in", skin.snake_in),
            ("snake-out", skin.snake_out),
            ("cursor-expand", skin.cursor_expand),
            ("cursor-trail", skin.cursor_trail),
            ("keypad", skin.keypad),
            ("key-bars", skin.key_bars),
            ("unstable-rate", skin.unstable_rate),
            ("hit-lighting", skin.hit_lighting),
            ("slider-ball-tint", skin.slider_ball_tint),
        ] {
            if set {
                on.push(name);
            }
        }
        on
    }
}
