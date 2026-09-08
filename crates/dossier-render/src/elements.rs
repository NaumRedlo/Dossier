use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Point,
    RadialGradient, Shader, SpreadMode, Stroke, Transform,
};

use crate::skin::{darken, lighten, with_alpha, ArrowShape};

pub(crate) fn dot(pixmap: &mut Pixmap, x: f32, y: f32, radius: f32, colour: Color, alpha: f32) {
    if radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let Some(path) = PathBuilder::from_circle(x, y, radius) else {
        return;
    };
    let paint = Paint {
        shader: Shader::SolidColor(with_alpha(colour, alpha)),
        anti_alias: true,
        ..Default::default()
    };
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

pub(crate) fn ring(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    radius: f32,
    width: f32,
    colour: Color,
    alpha: f32,
) {
    if radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let Some(path) = PathBuilder::from_circle(x, y, radius) else {
        return;
    };
    let paint = Paint {
        shader: Shader::SolidColor(with_alpha(colour, alpha)),
        anti_alias: true,
        ..Default::default()
    };
    let stroke = Stroke {
        width: width.max(0.5),
        ..Default::default()
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

pub(crate) fn lit_dot(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    radius: f32,
    colour: Color,
    alpha: f32,
    relief: f32,
) {
    if relief <= 0.0 {
        dot(pixmap, x, y, radius, colour, alpha);
        return;
    }
    if radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let Some(path) = PathBuilder::from_circle(x, y, radius) else {
        return;
    };

    let light = Point::from_xy(x - radius * 0.22, y - radius * 0.30);
    let stops = vec![
        GradientStop::new(0.0, with_alpha(lighten(colour, relief), alpha)),
        GradientStop::new(0.55, with_alpha(colour, alpha)),
        GradientStop::new(1.0, with_alpha(darken(colour, relief * 0.5), alpha)),
    ];
    let shader = RadialGradient::new(
        light,
        Point::from_xy(x, y),
        radius * 1.15,
        stops,
        SpreadMode::Pad,
        Transform::identity(),
    );
    let Some(shader) = shader else {
        dot(pixmap, x, y, radius, colour, alpha);
        return;
    };
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Default::default()
    };
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

pub(crate) fn glow(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    radius: f32,
    colour: Color,
    alpha: f32,
    reach: f32,
) {
    if reach <= 0.0 || radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let outer = radius * (1.0 + reach);
    let Some(path) = PathBuilder::from_circle(x, y, outer) else {
        return;
    };

    let edge = (radius / outer).clamp(0.0, 1.0);
    let strength = alpha * 0.22;
    let stops = vec![
        GradientStop::new(0.0, with_alpha(colour, strength)),
        GradientStop::new(edge, with_alpha(colour, strength * 0.7)),
        GradientStop::new(1.0, with_alpha(colour, 0.0)),
    ];
    let shader = RadialGradient::new(
        Point::from_xy(x, y),
        Point::from_xy(x, y),
        outer,
        stops,
        SpreadMode::Pad,
        Transform::identity(),
    );
    let Some(shader) = shader else {
        return;
    };
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Default::default()
    };
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Element {
    HitCircle,

    HitCircleOverlay,

    ApproachCircle,

    ReverseArrow,

    SliderScorePoint,

    Lighting,

    InputOverlayBackground,
    InputOverlayKey,

    FollowPoint,

    SliderHead,
    SliderHeadOverlay,
    SliderTail,
    SliderTailOverlay,

    SliderBall,

    SliderFollowCircle,

    Cursor,

    CursorMiddle,

    CursorTrail,

    Verdict(Verdict),

    SpinnerCircle,
    SpinnerMiddle,

    SpinnerMiddle2,

    SpinnerBackground,

    SpinnerMetre,

    SpinnerBottom,
    SpinnerGlow,
    SpinnerTop,

    SpinnerRpm,

    SpinnerSpin,
    SpinnerClear,

    SectionPass,
    SectionFail,

    SpinnerApproachCircle,

    ScoreBarBackground,
    ScoreBarFill,
    ScoreBarMark(Health),

    Score(char),

    Combo(char),

    Digit(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Health {
    Fine,
    Low,
    Critical,
}

impl Health {
    pub fn of(fraction: f32) -> Self {
        if fraction < 0.2 {
            Self::Critical
        } else if fraction < 0.5 {
            Self::Low
        } else {
            Self::Fine
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Self::Fine => "scorebar-ki",
            Self::Low => "scorebar-kidanger",
            Self::Critical => "scorebar-kidanger2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Miss,
    Fifty,
    Hundred,
    HundredKatu,
    Three,
    ThreeKatu,
    ThreeGeki,
}

impl Verdict {
    pub const ALL: [Self; 7] = [
        Self::Miss,
        Self::Fifty,
        Self::Hundred,
        Self::HundredKatu,
        Self::Three,
        Self::ThreeKatu,
        Self::ThreeGeki,
    ];

    fn stem(self) -> &'static str {
        match self {
            Self::Miss => "hit0",
            Self::Fifty => "hit50",
            Self::Hundred => "hit100",
            Self::HundredKatu => "hit100k",
            Self::Three => "hit300",
            Self::ThreeKatu => "hit300k",
            Self::ThreeGeki => "hit300g",
        }
    }

    fn mark(self) -> (&'static str, f32) {
        match self {
            Self::Miss => ("×", 0.85),
            Self::Fifty => ("50", 0.46),
            Self::Hundred | Self::HundredKatu => ("100", 0.42),
            Self::Three | Self::ThreeKatu | Self::ThreeGeki => ("300", 0.42),
        }
    }
}

pub const DIGIT_PADDING: f32 = 4.0;

const DIGIT_REFERENCE: f32 = 128.0;

impl Element {
    pub fn stem(self) -> String {
        match self {
            Self::HitCircle => "hitcircle".to_owned(),
            Self::HitCircleOverlay => "hitcircleoverlay".to_owned(),
            Self::ApproachCircle => "approachcircle".to_owned(),
            Self::ReverseArrow => "reversearrow".to_owned(),
            Self::SliderScorePoint => "sliderscorepoint".to_owned(),
            Self::InputOverlayBackground => "inputoverlay-background".to_owned(),
            Self::InputOverlayKey => "inputoverlay-key".to_owned(),
            Self::FollowPoint => "followpoint".to_owned(),
            Self::Lighting => "lighting".to_owned(),
            Self::SliderHead => "sliderstartcircle".to_owned(),
            Self::SliderHeadOverlay => "sliderstartcircleoverlay".to_owned(),
            Self::SliderTail => "sliderendcircle".to_owned(),
            Self::SliderTailOverlay => "sliderendcircleoverlay".to_owned(),
            Self::SliderBall => "sliderb".to_owned(),
            Self::SliderFollowCircle => "sliderfollowcircle".to_owned(),
            Self::Cursor => "cursor".to_owned(),
            Self::CursorMiddle => "cursormiddle".to_owned(),
            Self::CursorTrail => "cursortrail".to_owned(),
            Self::Verdict(v) => v.stem().to_owned(),
            Self::SpinnerApproachCircle => "spinner-approachcircle".to_owned(),
            Self::SpinnerCircle => "spinner-circle".to_owned(),
            Self::SpinnerMiddle => "spinner-middle".to_owned(),
            Self::SpinnerMiddle2 => "spinner-middle2".to_owned(),
            Self::SpinnerBackground => "spinner-background".to_owned(),
            Self::SpinnerMetre => "spinner-metre".to_owned(),
            Self::SpinnerBottom => "spinner-bottom".to_owned(),
            Self::SpinnerGlow => "spinner-glow".to_owned(),
            Self::SpinnerTop => "spinner-top".to_owned(),
            Self::SpinnerRpm => "spinner-rpm".to_owned(),
            Self::SpinnerSpin => "spinner-spin".to_owned(),
            Self::SpinnerClear => "spinner-clear".to_owned(),
            Self::SectionPass => "section-pass".to_owned(),
            Self::SectionFail => "section-fail".to_owned(),
            Self::ScoreBarBackground => "scorebar-bg".to_owned(),
            Self::ScoreBarFill => "scorebar-colour".to_owned(),
            Self::ScoreBarMark(state) => state.stem().to_owned(),

            Self::Score(c) | Self::Combo(c) => match c {
                ',' => "score-comma".to_owned(),
                '.' => "score-dot".to_owned(),
                '%' => "score-percent".to_owned(),
                'x' => "score-x".to_owned(),
                other => format!("score-{other}"),
            },
            Self::Digit(n) => format!("default-{n}"),
        }
    }

    pub fn stem_with(self, ini: &crate::imported::Ini) -> String {
        match self {
            Self::Digit(n) => format!("{}-{n}", ini.hit_circle_prefix),
            Self::Score(c) | Self::Combo(c) => {
                let prefix = if matches!(self, Self::Combo(_)) {
                    &ini.combo_prefix
                } else {
                    &ini.score_prefix
                };
                match c {
                    ',' => format!("{prefix}-comma"),
                    '.' => format!("{prefix}-dot"),
                    '%' => format!("{prefix}-percent"),
                    'x' => format!("{prefix}-x"),
                    other => format!("{prefix}-{other}"),
                }
            }
            other => other.stem(),
        }
    }

    pub fn is_tinted(self) -> bool {
        matches!(
            self,
            Self::HitCircle
                | Self::ApproachCircle
                | Self::SliderBall
                | Self::SliderHead
                | Self::SliderTail
                | Self::Lighting
        )
    }

    pub fn size(self) -> u32 {
        match self {
            Self::HitCircle | Self::HitCircleOverlay | Self::ReverseArrow => 128,

            Self::SliderHead
            | Self::SliderHeadOverlay
            | Self::SliderTail
            | Self::SliderTailOverlay => 128,
            Self::ApproachCircle => 126,
            Self::SliderScorePoint => 16,
            Self::FollowPoint => 64,

            Self::InputOverlayKey => 46,
            Self::InputOverlayBackground => 64,

            Self::Lighting => 100,
            Self::SliderBall => 128,
            Self::SliderFollowCircle => 256,

            Self::Score(_) | Self::Combo(_) => 64,
            Self::ScoreBarBackground | Self::ScoreBarFill => 640,
            Self::SpinnerCircle | Self::SpinnerMiddle | Self::SpinnerMiddle2 => 666,
            Self::SpinnerBackground => 640,
            Self::SpinnerBottom | Self::SpinnerGlow | Self::SpinnerTop => 666,
            Self::SpinnerMetre => 1024,
            Self::SpinnerRpm => 256,
            Self::SpinnerSpin | Self::SpinnerClear => 512,
            Self::SectionPass | Self::SectionFail => 800,
            Self::ScoreBarMark(_) => 160,
            Self::Cursor | Self::CursorMiddle => 128,
            Self::CursorTrail => 64,
            Self::SpinnerApproachCircle => 384,
            Self::Verdict(_) | Self::Digit(_) => DIGIT_REFERENCE as u32,
        }
    }
}

pub fn element(skin: &crate::skin::Skin, element: Element, size: u32) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(size, size)?;
    let half = size as f32 / 2.0;

    let white = Color::from_rgba8(255, 255, 255, 255);
    match element {
        Element::HitCircle => {
            let radius = half * 0.94;
            let border = radius * skin.border_ratio;

            lit_dot(
                &mut pixmap,
                half,
                half,
                radius - border,
                white,
                1.0,
                skin.note_relief,
            );
        }
        Element::HitCircleOverlay => {
            let radius = half * 0.94;
            let border = radius * skin.border_ratio;
            ring(
                &mut pixmap,
                half,
                half,
                radius - border / 2.0,
                border,
                skin.circle_border,
                1.0,
            );
        }
        Element::ApproachCircle => {
            let width = (size as f32 * 0.035).max(2.0);
            ring(&mut pixmap, half, half, half - width, width, white, 1.0);
        }
        Element::ReverseArrow => {
            chevron(
                &mut pixmap,
                half,
                half,
                (1.0, 0.0),
                half * 0.62,
                skin.circle_border,
                1.0,
                skin.arrow,
                0.22,
            );
        }
        Element::SliderScorePoint => {
            dot(
                &mut pixmap,
                half,
                half,
                half * 0.75,
                skin.circle_border,
                1.0,
            );
        }
        Element::Cursor => {
            let radius = half * 0.42;
            glow(&mut pixmap, half, half, radius, skin.trail_colour, 1.0, 0.9);
            dot(&mut pixmap, half, half, radius, skin.trail_colour, 1.0);
        }
        Element::CursorMiddle => {
            dot(&mut pixmap, half, half, half * 0.25, skin.cursor, 1.0);
        }
        Element::CursorTrail => {
            dot(&mut pixmap, half, half, half * 0.5, skin.trail_colour, 0.55);
        }
        Element::SpinnerApproachCircle => {
            let width = (size as f32 * 0.02).max(2.0);
            ring(
                &mut pixmap,
                half,
                half,
                half - width,
                width,
                skin.spinner,
                1.0,
            );
        }

        Element::SliderBall
        | Element::SliderFollowCircle
        | Element::FollowPoint
        | Element::Lighting
        | Element::InputOverlayBackground
        | Element::InputOverlayKey
        | Element::SliderHead
        | Element::SliderHeadOverlay
        | Element::SliderTail
        | Element::SliderTailOverlay
        | Element::Score(_)
        | Element::Combo(_)
        | Element::ScoreBarBackground
        | Element::ScoreBarFill
        | Element::ScoreBarMark(_)
        | Element::SpinnerCircle
        | Element::SpinnerMiddle
        | Element::SpinnerMiddle2
        | Element::SpinnerBackground
        | Element::SpinnerMetre
        | Element::SpinnerBottom
        | Element::SpinnerGlow
        | Element::SpinnerTop
        | Element::SpinnerRpm
        | Element::SpinnerSpin
        | Element::SpinnerClear
        | Element::SectionPass
        | Element::SectionFail => return None,
        Element::Verdict(_) | Element::Digit(_) => return lettered(skin, element, size),
    }
    Some(pixmap)
}

fn lettered(skin: &crate::skin::Skin, element: Element, size: u32) -> Option<Pixmap> {
    let font = skin.font.as_ref()?;
    let (text, share, colour) = match element {
        Element::Digit(value) => (value.to_string(), 0.47 / 0.8, skin.circle_border),
        Element::Verdict(verdict) => {
            if matches!(
                verdict,
                Verdict::Three | Verdict::ThreeKatu | Verdict::ThreeGeki
            ) && !skin.show_300
            {
                return Pixmap::new(size / 4, size / 4);
            }
            let (text, scale) = verdict.mark();
            let colour = match verdict {
                Verdict::Miss => skin.verdict_miss,
                Verdict::Fifty => skin.verdict_50,
                Verdict::Hundred | Verdict::HundredKatu => skin.verdict_100,
                Verdict::Three | Verdict::ThreeKatu | Verdict::ThreeGeki => skin.verdict_300,
            };

            (text.to_owned(), scale / 2.0, colour)
        }
        _ => return None,
    };

    let scale = size as f32 / DIGIT_REFERENCE;

    let plain = DIGIT_REFERENCE * share;
    let unit_width = (font.width(&text, plain) + DIGIT_PADDING * 2.0).ceil();
    let unit_height = (font.digit_height(plain) + DIGIT_PADDING * 2.0).ceil();
    let mut pixmap = Pixmap::new(
        (unit_width * scale).round() as u32,
        (unit_height * scale).round() as u32,
    )?;
    font.draw(
        &mut pixmap,
        crate::text::Label {
            text: &text,
            x: unit_width * scale / 2.0,
            y: DIGIT_PADDING * scale + font.digit_height(plain * scale),
            size: plain * scale,
            colour,
            align: crate::text::Align::Centre,
        },
    );
    Some(pixmap)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn chevron(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    dir: (f64, f64),
    size: f32,
    colour: Color,
    alpha: f32,
    shape: ArrowShape,
    rounding: f32,
) {
    let (dx, dy) = dir;
    let (px, py) = (-dy, dx);
    let point = |along: f64, across: f64| {
        (
            x + (dx * along + px * across) as f32 * size,
            y + (dy * along + py * across) as f32 * size,
        )
    };

    let outline: &[(f64, f64)] = match shape {
        ArrowShape::Triangle | ArrowShape::Rounded => &[(1.0, 0.0), (-0.55, 0.85), (-0.55, -0.85)],
    };

    let mut builder = PathBuilder::with_capacity(outline.len() + 1, outline.len() + 1);
    let (first_x, first_y) = point(outline[0].0, outline[0].1);
    builder.move_to(first_x, first_y);
    for &(along, across) in &outline[1..] {
        let (px, py) = point(along, across);
        builder.line_to(px, py);
    }
    builder.close();
    let Some(path) = builder.finish() else {
        return;
    };

    let paint = Paint {
        shader: Shader::SolidColor(with_alpha(colour, alpha)),
        anti_alias: true,
        ..Default::default()
    };
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    if shape != ArrowShape::Triangle {
        let stroke = Stroke {
            width: size * rounding,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

#[cfg(test)]
mod health_marks {
    use super::*;

    #[test]
    fn the_mark_changes_at_the_thresholds_the_game_uses() {
        assert_eq!(Health::of(1.0), Health::Fine);
        assert_eq!(Health::of(0.5), Health::Fine);
        assert_eq!(Health::of(0.49), Health::Low);
        assert_eq!(Health::of(0.2), Health::Low);
        assert_eq!(Health::of(0.19), Health::Critical);
        assert_eq!(Health::of(0.0), Health::Critical);
    }

    #[test]
    fn each_mark_reads_its_own_file() {
        let names: Vec<String> = [Health::Fine, Health::Low, Health::Critical]
            .map(|h| Element::ScoreBarMark(h).stem())
            .to_vec();
        assert_eq!(
            names,
            ["scorebar-ki", "scorebar-kidanger", "scorebar-kidanger2"]
        );
    }

    #[test]
    fn the_bar_is_three_separate_files() {
        assert_eq!(Element::ScoreBarBackground.stem(), "scorebar-bg");
        assert_eq!(Element::ScoreBarFill.stem(), "scorebar-colour");
    }
}
