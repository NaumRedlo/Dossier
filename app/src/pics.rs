use std::path::Path;

use dossier_render::elements::{Element, Verdict};
use dossier_render::imported::Sprites;

#[derive(Default, serde::Serialize)]
pub struct Pictures {
    pub circle: Option<Picture>,
    pub overlay: Option<Picture>,
    pub approach: Option<Picture>,
    pub cursor: Option<Picture>,

    pub cursor_middle: Option<Picture>,

    pub cursor_trail: Option<Picture>,

    pub slider_head: Option<Picture>,
    pub slider_head_overlay: Option<Picture>,
    pub slider_tail: Option<Picture>,
    pub slider_tail_overlay: Option<Picture>,

    pub slider_ball: Option<Picture>,
    pub follow_circle: Option<Picture>,
    pub score_point: Option<Picture>,
    pub reverse_arrow: Option<Picture>,

    pub lighting: Option<Picture>,
    pub follow_point: Option<Picture>,

    pub digits: Vec<Picture>,

    pub verdicts: Verdicts,

    pub colours: Vec<String>,

    pub rules: Rules,
}

#[derive(Default, serde::Serialize)]
pub struct Rules {
    pub overlay_above_number: bool,
    pub hit_circle_overlap: f32,
    pub cursor_rotate: bool,
    pub cursor_expand: bool,
    pub slider_ball_tint: bool,

    pub slider_border: Option<String>,
    pub slider_track: Option<String>,

    pub number_swells: bool,
}

#[derive(Default, serde::Serialize)]
pub struct Verdicts {
    pub miss: Option<Picture>,
    pub fifty: Option<Picture>,
    pub hundred: Option<Picture>,
    pub three: Option<Picture>,
}

#[derive(serde::Serialize)]
pub struct Picture {
    pub src: String,

    pub scale: f32,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - i * 6)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn picture(sprites: &Sprites, element: Element) -> Option<Picture> {
    let sprite = sprites.get(element)?;
    let png = sprite.pixmap.encode_png().ok()?;
    Some(Picture {
        src: format!("data:image/png;base64,{}", base64(&png)),
        scale: sprite.scale,
    })
}

pub fn of(folder: &Path) -> Pictures {
    let wanted: Vec<Element> = [
        Element::HitCircle,
        Element::HitCircleOverlay,
        Element::ApproachCircle,
        Element::Cursor,
        Element::CursorMiddle,
        Element::CursorTrail,
        Element::SliderHead,
        Element::SliderHeadOverlay,
        Element::SliderTail,
        Element::SliderTailOverlay,
        Element::SliderBall,
        Element::SliderFollowCircle,
        Element::SliderScorePoint,
        Element::ReverseArrow,
        Element::Lighting,
        Element::FollowPoint,
    ]
    .into_iter()
    .chain((0..10).map(Element::Digit))
    .chain(
        [
            Verdict::Miss,
            Verdict::Fifty,
            Verdict::Hundred,
            Verdict::Three,
        ]
        .into_iter()
        .map(Element::Verdict),
    )
    .collect();
    let sprites = Sprites::read(folder, &wanted);

    let digits: Vec<Picture> = (0..10)
        .filter_map(|n| picture(&sprites, Element::Digit(n)))
        .collect();
    let ini = sprites.ini();
    Pictures {
        circle: picture(&sprites, Element::HitCircle),
        overlay: picture(&sprites, Element::HitCircleOverlay),
        approach: picture(&sprites, Element::ApproachCircle),
        cursor: picture(&sprites, Element::Cursor),
        cursor_middle: picture(&sprites, Element::CursorMiddle),
        cursor_trail: picture(&sprites, Element::CursorTrail),

        slider_head: picture(&sprites, Element::SliderHead),
        slider_head_overlay: picture(&sprites, Element::SliderHeadOverlay),
        slider_tail: picture(&sprites, Element::SliderTail),
        slider_tail_overlay: picture(&sprites, Element::SliderTailOverlay),
        slider_ball: picture(&sprites, Element::SliderBall),
        follow_circle: picture(&sprites, Element::SliderFollowCircle),
        score_point: picture(&sprites, Element::SliderScorePoint),
        reverse_arrow: picture(&sprites, Element::ReverseArrow),
        lighting: picture(&sprites, Element::Lighting),
        follow_point: picture(&sprites, Element::FollowPoint),
        rules: Rules {
            overlay_above_number: ini.overlay_above_number,
            hit_circle_overlap: ini.hit_circle_overlap,
            cursor_rotate: ini.cursor_rotate,
            cursor_expand: ini.cursor_expand,
            slider_ball_tint: ini.slider_ball_tint,
            slider_border: ini.slider_border.map(|c| hex(c.red(), c.green(), c.blue())),
            slider_track: ini.slider_track.map(|c| hex(c.red(), c.green(), c.blue())),

            number_swells: ini.version < 2.0,
        },
        digits: if digits.len() == 10 {
            digits
        } else {
            Vec::new()
        },
        verdicts: Verdicts {
            miss: picture(&sprites, Element::Verdict(Verdict::Miss)),
            fifty: picture(&sprites, Element::Verdict(Verdict::Fifty)),
            hundred: picture(&sprites, Element::Verdict(Verdict::Hundred)),
            three: picture(&sprites, Element::Verdict(Verdict::Three)),
        },
        colours: ini
            .combo_colours
            .iter()
            .map(|c| hex(c.red(), c.green(), c.blue()))
            .collect(),
    }
}

fn hex(red: f32, green: f32, blue: f32) -> String {
    let byte = |part: f32| (part.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(red), byte(green), byte(blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_says_what_everybody_elses_says() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn a_folder_that_is_not_a_skin_gives_nothing_rather_than_failing() {
        let said = of(Path::new("/definitely/not/here"));
        assert!(said.circle.is_none());
        assert!(said.digits.is_empty());
    }
}
