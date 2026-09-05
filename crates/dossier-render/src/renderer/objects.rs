use super::*;

use dossier_beatmap::Point;
use dossier_sim::{GameState, TimedKind, TimedObject};
use tiny_skia::{
    Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, Shader, Stroke, Transform,
};

use crate::elements::Element;
use crate::layout::Layout;
use crate::skin::{darken, with_alpha, ArrowShape};

const SKIN_CIRCLE_PIXELS: f32 = 128.0;

const DIGIT_SCALE: f32 = 0.8;
const DIGIT_MAX_PIXELS: f32 = 64.0 * 2.0 / DIGIT_SCALE;

const SHADOW_PORTION: f32 = 1.0 - 59.0 / 64.0;
const BORDER_PORTION: f32 = 0.1875;

const SHADOW_ALPHA: f32 = 0.25;

fn tube_shade(
    towards: f32,
    border: Color,
    body_outer: Color,
    body_inner: Color,
    body_alpha: f32,
) -> Color {
    if towards <= SHADOW_PORTION {
        return with_alpha(
            Color::from_rgba8(0, 0, 0, 255),
            SHADOW_ALPHA * towards / SHADOW_PORTION,
        );
    }
    if towards <= BORDER_PORTION {
        return border;
    }
    let along = ((towards - BORDER_PORTION) / (1.0 - BORDER_PORTION)).clamp(0.0, 1.0);
    with_alpha(blend(body_outer, body_inner, along), body_alpha)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Face {
    Note,
    Head,
    Tail,
}

impl Scene<'_> {
    #[doc(hidden)]
    pub fn alpha_for_test(&self, index: usize, time_ms: f64) -> f32 {
        self.alpha_of(index, time_ms)
    }

    #[doc(hidden)]
    pub fn head_alpha_for_test(&self, index: usize, time_ms: f64) -> f32 {
        self.head_alpha(index, time_ms)
    }

    pub(super) fn alpha_of(&self, index: usize, time_ms: f64) -> f32 {
        self.alpha_at(index, time_ms, HiddenFade::Own)
    }

    fn alpha_at(&self, index: usize, time_ms: f64, hidden: HiddenFade) -> f32 {
        let annotation = &self.annotations[index];
        if time_ms < annotation.spawn_ms || time_ms > annotation.gone_ms {
            return 0.0;
        }

        let leaves = annotation.gone_ms - HIT_FADE_MS;
        let fade_in = if self.hidden {
            self.state.difficulty().preempt_ms() * HIDDEN_FADE_IN
        } else {
            self.state.difficulty().fade_in_ms()
        }
        .max(1.0);
        let appearing = ((time_ms - annotation.spawn_ms) / fade_in).clamp(0.0, 1.0) as f32;

        let leaving = 1.0 - (((time_ms - leaves) / HIT_FADE_MS).clamp(0.0, 1.0)) as f32;

        let object = &self.state.timeline().objects[index];
        if self.hidden && hidden != HiddenFade::Untouched && !object.is_spinner() {
            let starts = annotation.spawn_ms + fade_in;
            let duration = if object.is_slider() && hidden == HiddenFade::Own {
                (object.end_ms - starts).max(1.0)
            } else {
                self.state.difficulty().preempt_ms() * HIDDEN_FADE_OUT
            };
            let hiding = 1.0 - (((time_ms - starts) / duration).clamp(0.0, 1.0) as f32);
            return appearing * leaving * hiding;
        }
        appearing * leaving
    }

    fn alpha_through_hidden(&self, index: usize, time_ms: f64) -> f32 {
        self.alpha_at(index, time_ms, HiddenFade::Untouched)
    }

    fn head_alpha(&self, index: usize, time_ms: f64) -> f32 {
        self.alpha_at(index, time_ms, HiddenFade::AsANote)
    }

    fn exit_progress(&self, from_ms: f64, time_ms: f64, missed: bool) -> f32 {
        let over = if missed { MISS_FADE_MS } else { HIT_FADE_MS };
        (((time_ms - from_ms) / over).clamp(0.0, 1.0)) as f32
    }

    fn number_alpha(&self, from_ms: f64, time_ms: f64) -> f32 {
        let over = if self.number_swells() {
            HIT_FADE_MS
        } else {
            NUMBER_FADE_MS
        };
        (1.0 - ((time_ms - from_ms) / over).clamp(0.0, 1.0)) as f32
    }

    fn number_swells(&self) -> bool {
        self.skin_version() <= 1.0
    }

    fn snake(&self, object: &TimedObject, index: usize, time_ms: f64) -> (f64, f64) {
        let TimedKind::Slider { slides, .. } = &object.kind else {
            return (0.0, 1.0);
        };
        let annotation = &self.annotations[index];

        if time_ms < object.start_ms {
            if !self.skin.snake_in {
                return (0.0, 1.0);
            }

            let approach = (object.start_ms - annotation.spawn_ms).max(1.0);
            let window = approach * SNAKE_SHARE_OF_APPROACH;
            return (
                0.0,
                ((time_ms - annotation.spawn_ms) / window).clamp(0.0, 1.0),
            );
        }
        if !self.skin.snake_out {
            return (0.0, 1.0);
        }

        let slides = (*slides).max(1);
        let span = (object.end_ms - object.start_ms).max(1.0);
        let travelled =
            ((time_ms - object.start_ms) / span * f64::from(slides)).clamp(0.0, f64::from(slides));
        let last = f64::from(slides - 1);
        if travelled < last {
            return (0.0, 1.0);
        }

        let local = (travelled - last).clamp(0.0, 1.0);
        if slides % 2 == 1 {
            (local, 1.0)
        } else {
            (0.0, 1.0 - local)
        }
    }

    pub(super) fn draw_object_body(
        &self,
        pixmap: &mut Pixmap,
        index: usize,
        time_ms: f64,
        layout: &Layout,
    ) {
        let object = &self.state.timeline().objects[index];
        if !matches!(object.kind, TimedKind::Slider { .. }) {
            return;
        }
        let annotation = &self.annotations[index];
        let colour = self.skin.combo_colour(annotation.colour);
        let (from, to) = self.snake(object, index, time_ms);
        self.draw_slider_body(
            pixmap,
            object,
            (from, to),
            colour,
            self.alpha_of(index, time_ms),
            layout,
        );
    }

    pub(super) fn draw_approach(
        &self,
        pixmap: &mut Pixmap,
        index: usize,
        time_ms: f64,
        layout: &Layout,
    ) {
        let object = &self.state.timeline().objects[index];

        if object.is_spinner() || time_ms >= object.start_ms || self.hidden {
            return;
        }
        let alpha = self.alpha_of(index, time_ms);
        if alpha <= 0.0 {
            return;
        }
        let annotation = &self.annotations[index];
        let radius = layout.length(self.state.difficulty().circle_radius());
        let progress = self.state.timeline().approach_progress(object, time_ms);
        let scale = 1.0 + 3.0 * (1.0 - progress.clamp(0.0, 1.0)) as f32;

        if self.skin_speaks_for(Element::ApproachCircle) {
            self.draw_sprite(
                pixmap,
                Element::ApproachCircle,
                annotation.colour,
                object.pos,
                radius * scale,
                alpha,
                layout,
            );
        } else {
            self.ring(
                pixmap,
                object.pos,
                radius * scale,
                (radius * 0.09).max(1.0),
                self.skin.combo_colour(annotation.colour),
                alpha,
                layout,
            );
        }
    }

    pub(super) fn draw_object(
        &self,
        pixmap: &mut Pixmap,
        index: usize,
        time_ms: f64,
        layout: &Layout,
    ) {
        let object = &self.state.timeline().objects[index];
        let annotation = &self.annotations[index];
        let alpha = self.alpha_of(index, time_ms);
        let colour = self.skin.combo_colour(annotation.colour);
        let radius = layout.length(self.state.difficulty().circle_radius());

        match &object.kind {
            TimedKind::Spinner => self.draw_spinner(pixmap, object, time_ms, alpha, layout),
            TimedKind::Slider { .. } => {
                self.draw_object_body(pixmap, index, time_ms, layout);
                let (from, to) = self.snake(object, index, time_ms);
                let slide = object.slide_duration_ms().unwrap_or(0.0);

                if to >= 1.0 {
                    if let Some(end) = object.ball_at(object.start_ms + slide) {
                        let at = shaken(end, annotation, time_ms, self.state);
                        self.draw_circle(
                            pixmap,
                            at,
                            radius,
                            colour,
                            alpha,
                            layout,
                            annotation.colour,
                            Face::Tail,
                        );
                    }
                }
                for &tick in &annotation.ticks_ms {
                    let on_body =
                        path_fraction(object, tick).is_some_and(|frac| frac >= from && frac <= to);
                    if tick <= time_ms || !on_body {
                        continue;
                    }
                    let Some(at) = object.ball_at(tick) else {
                        continue;
                    };

                    let span = if slide > 0.0 {
                        ((tick - object.start_ms) / slide).floor()
                    } else {
                        0.0
                    };
                    let offset = if span > 0.0 {
                        TICK_REPEAT_LEAD_MS
                    } else {
                        self.state.difficulty().preempt_ms() * TICK_FIRST_LEAD
                    };
                    let live = tick - ((tick - (object.start_ms + span * slide)) / 2.0 + offset);
                    let arriving = (((time_ms - live) / TICK_FADE_MS).clamp(0.0, 1.0)) as f32;
                    if arriving <= 0.0 {
                        continue;
                    }

                    let grown = 0.5
                        + 0.5
                            * fade(
                                (((time_ms - live) / (TICK_FADE_MS * 4.0)).clamp(0.0, 1.0)) as f32,
                            );

                    if self.skin_speaks_for(Element::SliderScorePoint) {
                        self.draw_sprite(
                            pixmap,
                            Element::SliderScorePoint,
                            annotation.colour,
                            at,
                            radius,
                            alpha * arriving * grown,
                            layout,
                        );
                    } else {
                        self.dot(
                            pixmap,
                            at,
                            radius * 0.14 * grown,
                            lighten(self.skin.circle_border, 0.5),
                            alpha * arriving,
                            layout,
                        );
                    }
                }

                let carried = self.alpha_through_hidden(index, time_ms);

                let leaving = ((time_ms - object.end_ms) / FOLLOW_LEAVE_MS).clamp(0.0, 1.0);
                let held = object.ball_at(time_ms.min(object.end_ms));
                if let Some(ball) = held.filter(|_| time_ms >= object.start_ms && leaving < 1.0) {
                    let going = 1.0 - leaving as f32;

                    let beat = self.follow_pulse(index, time_ms.min(object.end_ms))
                        * (FOLLOW_LEAVE_TO + (1.0 - FOLLOW_LEAVE_TO) * going);
                    let carried = carried * going;
                    if self.skin_speaks_for(Element::SliderFollowCircle) {
                        self.draw_sprite(
                            pixmap,
                            Element::SliderFollowCircle,
                            annotation.colour,
                            ball,
                            radius * beat,
                            carried,
                            layout,
                        );
                    } else {
                        self.ring(
                            pixmap,
                            ball,
                            radius * 2.4 * beat,
                            radius * 0.06,
                            self.skin.circle_border,
                            carried * 0.5,
                            layout,
                        );
                    }

                    if leaving == 0.0 {
                        let done = ((time_ms - object.start_ms)
                            / (object.end_ms - object.start_ms).max(1.0))
                        .clamp(0.0, 1.0) as f32;
                        if self.skin_speaks_for(Element::SliderBall) {
                            let _ = done;

                            if self.ball_is_mirrored(object, time_ms) {
                                self.draw_sprite_mirrored(
                                    pixmap,
                                    Element::SliderBall,
                                    annotation.colour,
                                    ball,
                                    radius,
                                    carried,
                                    layout,
                                );
                            } else {
                                self.draw_sprite(
                                    pixmap,
                                    Element::SliderBall,
                                    annotation.colour,
                                    ball,
                                    radius,
                                    carried,
                                    layout,
                                );
                            }
                        } else {
                            self.dot(pixmap, ball, radius, colour, carried, layout);
                            self.dot(
                                pixmap,
                                ball,
                                radius * (BALL_CORE_SCALE + (1.0 - BALL_CORE_SCALE) * done),
                                lighten(colour, 0.45),
                                carried,
                                layout,
                            );
                        }
                    }
                }
                self.draw_reverse_arrow(
                    pixmap,
                    object,
                    annotation,
                    time_ms,
                    radius,
                    carried,
                    (from, to),
                    layout,
                );

                let exit = self.exit_progress(annotation.head_ms, time_ms, annotation.head_missed);
                if exit < 1.0 {
                    let leaving = self.head_alpha(index, time_ms) * (1.0 - exit);
                    let grown = radius * hit_expansion(exit, annotation.head_missed);
                    let at = shaken(object.pos, annotation, time_ms, self.state);
                    self.draw_circle(
                        pixmap,
                        at,
                        grown,
                        colour,
                        leaving,
                        layout,
                        annotation.colour,
                        Face::Head,
                    );

                    let showing = leaving * self.number_alpha(annotation.head_ms, time_ms);
                    if showing > 0.0 {
                        let worn = if self.number_swells() { grown } else { radius };
                        self.draw_number(pixmap, at, worn, annotation.number, showing, layout);
                    }
                    self.draw_rim(
                        pixmap,
                        at,
                        grown,
                        leaving,
                        layout,
                        annotation.colour,
                        Face::Head,
                    );
                }
            }
            TimedKind::Circle => {
                let exit = self.exit_progress(annotation.resolved_ms, time_ms, annotation.missed);
                let grown = radius * hit_expansion(exit, annotation.missed);
                let at = shaken(object.pos, annotation, time_ms, self.state);
                self.draw_circle(
                    pixmap,
                    at,
                    grown,
                    colour,
                    alpha,
                    layout,
                    annotation.colour,
                    Face::Note,
                );
                let showing = alpha * self.number_alpha(annotation.resolved_ms, time_ms);
                if showing > 0.0 {
                    let worn = if self.number_swells() { grown } else { radius };
                    self.draw_number(pixmap, at, worn, annotation.number, showing, layout);
                }
                self.draw_rim(
                    pixmap,
                    at,
                    grown,
                    alpha,
                    layout,
                    annotation.colour,
                    Face::Note,
                );
            }
        }

        if annotation.missed && time_ms > annotation.resolved_ms {
            self.ring(
                pixmap,
                object.pos,
                radius,
                radius * 0.18,
                self.skin.spinner,
                alpha * 0.7,
                layout,
            );
        }
    }

    fn face_of(&self, face: Face) -> Option<(Element, Element)> {
        let own = match face {
            Face::Note => None,
            Face::Head => Some((Element::SliderHead, Element::SliderHeadOverlay)),
            Face::Tail => Some((Element::SliderTail, Element::SliderTailOverlay)),
        };
        if let Some(pair) = own {
            if self.skin_speaks_for(pair.0) {
                return Some(pair);
            }
        }

        self.skin_speaks_for(Element::HitCircle)
            .then_some((Element::HitCircle, Element::HitCircleOverlay))
    }

    fn overlay_above_number(&self) -> bool {
        self.skin
            .sprites
            .as_ref()
            .is_none_or(|s| s.ini().overlay_above_number)
    }

    fn draw_rim(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        alpha: f32,
        layout: &Layout,
        combo: usize,
        face: Face,
    ) {
        if !self.overlay_above_number() {
            return;
        }
        let Some((_, overlay)) = self.face_of(face) else {
            return;
        };
        if self.skin_speaks_for(overlay) {
            self.draw_sprite(pixmap, overlay, combo, centre, radius, alpha, layout);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_circle(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        colour: tiny_skia::Color,
        alpha: f32,
        layout: &Layout,
        combo: usize,
        face: Face,
    ) {
        if let Some((disc, overlay)) = self.face_of(face) {
            self.draw_sprite(pixmap, disc, combo, centre, radius, alpha, layout);

            if !self.overlay_above_number() && self.skin_speaks_for(overlay) {
                self.draw_sprite(pixmap, overlay, combo, centre, radius, alpha, layout);
            }
            return;
        }
        if face == Face::Tail {
            return;
        }

        let border = radius * self.skin.border_ratio;

        self.glow(pixmap, centre, radius, colour, alpha, layout);
        self.dot(pixmap, centre, radius, darken(colour, 0.25), alpha, layout);
        self.lit_dot(pixmap, centre, radius - border, colour, alpha, layout);
        self.ring(
            pixmap,
            centre,
            radius - border / 2.0,
            border,
            self.skin.circle_border,
            alpha,
            layout,
        );
    }

    pub(super) fn skin_version(&self) -> f32 {
        let stated = self
            .skin
            .sprites
            .as_ref()
            .map_or(crate::imported::LATEST_SKIN_VERSION, |s| s.ini().version);
        crate::imported::effective_version(stated, self.skin.skin_version_as_written)
    }

    pub(super) fn skin_speaks_for(&self, element: Element) -> bool {
        self.skin
            .sprites
            .as_ref()
            .is_some_and(|s| !s.draw_ourselves(element))
    }

    fn ball_is_mirrored(&self, object: &TimedObject, time_ms: f64) -> bool {
        if !self
            .skin
            .sprites
            .as_ref()
            .is_some_and(|sprites| sprites.ini().slider_ball_flip)
        {
            return false;
        }
        let Some(slide) = object.slide_duration_ms().filter(|ms| *ms > 0.0) else {
            return false;
        };

        let last = (object.end_ms - object.start_ms - 1e-6).max(0.0);
        let along = (time_ms - object.start_ms).clamp(0.0, last);
        (along / slide).floor() as i64 % 2 == 1
    }

    pub(super) fn draw_sprite(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        combo: usize,
        centre: Point,
        radius: f32,
        alpha: f32,
        layout: &Layout,
    ) {
        self.draw_sprite_turned(pixmap, element, combo, centre, radius, alpha, layout, 0.0);
    }

    pub(super) fn skin_pixels(&self, layout: &Layout, own: f32) -> f32 {
        own * layout.height as f32 / 768.0
    }

    pub(super) fn draw_sprite_wide(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        centre: Point,
        width: f32,
        alpha: f32,
        layout: &Layout,
    ) {
        self.draw_wide(pixmap, element, centre, width, alpha, layout, 0.0, 0);
    }

    pub(super) fn draw_sprite_wide_turned(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        centre: Point,
        width: f32,
        alpha: f32,
        layout: &Layout,
        degrees: f32,
    ) {
        self.draw_wide(pixmap, element, centre, width, alpha, layout, degrees, 0);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_sprite_wide_at(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        centre: Point,
        width: f32,
        alpha: f32,
        layout: &Layout,
        elapsed_ms: f64,
    ) {
        let frame = self.animation_frame_once(element, elapsed_ms);
        self.draw_wide(pixmap, element, centre, width, alpha, layout, 0.0, frame);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_wide(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        centre: Point,
        width: f32,
        alpha: f32,
        layout: &Layout,
        degrees: f32,
        frame: usize,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };

        let picture = (frame > 0)
            .then(|| sprites.frame(element, frame))
            .flatten()
            .or_else(|| sprites.coloured(element, 0));
        let Some((art, per_osu_pixel)) = picture else {
            return;
        };
        if alpha <= 0.0 || width <= 0.0 {
            return;
        }
        let own = (art.width() as f32 / per_osu_pixel).max(1.0);
        let scale = width / (own * per_osu_pixel);
        let (x, y) = layout.map(centre);
        let transform = Transform::from_translate(x, y)
            .pre_rotate(degrees)
            .pre_scale(scale, scale)
            .pre_translate(-(art.width() as f32) / 2.0, -(art.height() as f32) / 2.0);
        pixmap.draw_pixmap(
            0,
            0,
            art.as_ref(),
            &PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            transform,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_sprite_turned(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        combo: usize,
        centre: Point,
        radius: f32,
        alpha: f32,
        layout: &Layout,
        degrees: f32,
    ) {
        self.draw_sprite_blended(
            pixmap,
            element,
            combo,
            centre,
            radius,
            alpha,
            layout,
            degrees,
            tiny_skia::BlendMode::SourceOver,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_sprite_mirrored(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        combo: usize,
        centre: Point,
        radius: f32,
        alpha: f32,
        layout: &Layout,
    ) {
        self.draw_sprite_blended(
            pixmap,
            element,
            combo,
            centre,
            radius,
            alpha,
            layout,
            0.0,
            tiny_skia::BlendMode::SourceOver,
            true,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_sprite_blended(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        combo: usize,
        centre: Point,
        radius: f32,
        alpha: f32,
        layout: &Layout,
        degrees: f32,
        blend_mode: tiny_skia::BlendMode,
        mirror: bool,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };
        let Some((art, per_osu_pixel)) = sprites.coloured(element, combo) else {
            return;
        };
        if alpha <= 0.0 {
            return;
        }

        let scale = (radius * 2.0) / (SKIN_CIRCLE_PIXELS * per_osu_pixel);
        let (x, y) = layout.map(centre);

        let across = if mirror { -scale } else { scale };
        let transform = Transform::from_translate(x, y)
            .pre_rotate(degrees)
            .pre_scale(across, scale)
            .pre_translate(-(art.width() as f32) / 2.0, -(art.height() as f32) / 2.0);
        pixmap.draw_pixmap(
            0,
            0,
            art.as_ref(),
            &PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                quality: tiny_skia::FilterQuality::Bilinear,
                blend_mode,
                ..Default::default()
            },
            transform,
            None,
        );
    }

    fn draw_number_from_skin(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        number: u32,
        alpha: f32,
        layout: &Layout,
    ) -> bool {
        let Some(sprites) = &self.skin.sprites else {
            return false;
        };
        let digits: Vec<u8> = number.to_string().bytes().map(|byte| byte - b'0').collect();
        if digits.iter().any(|&d| sprites.silenced(Element::Digit(d))) {
            return true;
        }
        let mut art = Vec::with_capacity(digits.len());
        for &digit in &digits {
            let Some(found) = sprites.coloured(Element::Digit(digit), 0) else {
                return false;
            };
            art.push(found);
        }

        let overlap = sprites.ini().hit_circle_overlap;

        let widths: Vec<f32> = art
            .iter()
            .map(|(pixmap, per)| (pixmap.width() as f32 / per).min(DIGIT_MAX_PIXELS))
            .collect();
        let total: f32 = widths.iter().sum::<f32>() - overlap * (digits.len() as f32 - 1.0);

        let scale = (radius * 2.0) / SKIN_CIRCLE_PIXELS * DIGIT_SCALE;
        let (cx, cy) = layout.map(centre);
        let mut pen = cx - total * scale / 2.0;
        for ((pixmap_of, per), width) in art.into_iter().zip(widths) {
            let each = scale / per;
            let height = pixmap_of.height() as f32 * each;
            let transform = Transform::from_translate(pen, cy - height / 2.0).pre_scale(each, each);
            pixmap.draw_pixmap(
                0,
                0,
                pixmap_of.as_ref(),
                &PixmapPaint {
                    opacity: alpha.clamp(0.0, 1.0),
                    quality: tiny_skia::FilterQuality::Bilinear,
                    ..Default::default()
                },
                transform,
                None,
            );
            pen += (width - overlap) * scale;
        }
        true
    }

    fn draw_number(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        number: u32,
        alpha: f32,
        layout: &Layout,
    ) {
        if self.draw_number_from_skin(pixmap, centre, radius, number, alpha, layout) {
            return;
        }
        let Some(font) = &self.skin.font else {
            return;
        };
        let size = radius * 0.9;
        let (x, y) = layout.map(centre);
        font.draw(
            pixmap,
            Label {
                text: &number.to_string(),
                x,
                y: y + font.digit_height(size) / 2.0,
                size,
                colour: with_alpha(self.skin.circle_border, alpha),
                align: Align::Centre,
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_reverse_arrow(
        &self,
        pixmap: &mut Pixmap,
        object: &TimedObject,
        annotation: &Annotation,
        time_ms: f64,
        radius: f32,
        alpha: f32,
        (from, to): (f64, f64),
        layout: &Layout,
    ) {
        let (
            Some((head, tail)),
            TimedKind::Slider {
                slides,
                slide_duration_ms,
                ..
            },
        ) = (annotation.turns, &object.kind)
        else {
            return;
        };

        if *slide_duration_ms <= 0.0 {
            return;
        }

        for (at_tail, turn) in [(true, tail), (false, head)] {
            let turns = (1..*slides)
                .filter(|k| k.is_multiple_of(2) != at_tail)
                .map(|k| {
                    (
                        object.start_ms + f64::from(k) * slide_duration_ms,
                        object.start_ms + f64::from(k - 1) * slide_duration_ms,
                    )
                });

            let turns: Vec<(f64, f64)> = turns.collect();

            let (leaving, pulse) = arrow_life(
                &turns,
                time_ms,
                time_ms.max(object.start_ms),
                object.start_ms,
                *slide_duration_ms,
            );

            let arriving = if at_tail {
                ((to - (1.0 - ARROW_REACH)) / ARROW_REACH).clamp(0.0, 1.0) as f32
            } else {
                ((ARROW_REACH - from) / ARROW_REACH).clamp(0.0, 1.0) as f32
            };

            let showing = alpha * leaving * arriving;
            if showing <= 0.0 {
                continue;
            }

            if self.skin_speaks_for(Element::ReverseArrow) {
                let mut degrees = turn.dir.1.atan2(turn.dir.0).to_degrees() as f32;

                if self.skin_version() <= 1.0 {
                    degrees += arrow_rock(time_ms, object.start_ms);
                }
                self.draw_sprite_turned(
                    pixmap,
                    Element::ReverseArrow,
                    annotation.colour,
                    turn.at,
                    radius * pulse,
                    showing,
                    layout,
                    degrees,
                );
                continue;
            }
            self.draw_chevron(
                pixmap,
                turn,
                radius * ARROW_SCALE * pulse,
                showing,
                self.skin.arrow,
                layout,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_chevron(
        &self,
        pixmap: &mut Pixmap,
        turn: Turn,
        size: f32,
        alpha: f32,
        shape: ArrowShape,
        layout: &Layout,
    ) {
        let (x, y) = layout.map(turn.at);
        crate::elements::chevron(
            pixmap,
            x,
            y,
            turn.dir,
            size,
            self.skin.circle_border,
            alpha,
            shape,
            ARROW_ROUNDING,
        );
    }

    fn draw_slider_body(
        &self,
        pixmap: &mut Pixmap,
        object: &TimedObject,
        snake: (f64, f64),
        colour: tiny_skia::Color,
        alpha: f32,
        layout: &Layout,
    ) {
        let Some(path) = body_path(object, snake) else {
            return;
        };

        let radius = self.state.difficulty().circle_radius() as f32;
        let half = layout.length(self.state.difficulty().circle_radius());
        if half < 0.5 || alpha <= 0.0 {
            return;
        }

        let bounds = path.bounds();
        let (x0, y0) = layout.map(Point {
            x: f64::from(bounds.left()),
            y: f64::from(bounds.top()),
        });
        let (x1, y1) = layout.map(Point {
            x: f64::from(bounds.right()),
            y: f64::from(bounds.bottom()),
        });
        let margin = half * 2.0 + 4.0;
        let (left, top) = (x0.min(x1) - margin, y0.min(y1) - margin);
        let width = ((x1 - x0).abs() + margin * 2.0).ceil() as u32;
        let height = ((y1 - y0).abs() + margin * 2.0).ceil() as u32;
        let Some(mut tube) = Pixmap::new(width.max(1), height.max(1)) else {
            return;
        };
        let into_tube = Transform::from_translate(-left, -top).pre_concat(layout.transform());

        let steps = ((half / 2.0).ceil() as usize).clamp(8, 48);
        let body = self.skin.slider_body.unwrap_or(colour);
        let (body_outer, body_inner) =
            (crate::skin::body_outer(body), crate::skin::body_inner(body));

        for step in (0..=steps).rev() {
            let towards = 1.0 - step as f32 / steps as f32;
            let shade = tube_shade(
                towards,
                self.skin.slider_border,
                body_outer,
                body_inner,
                self.skin.slider_body_alpha,
            );
            let paint = Paint {
                shader: Shader::SolidColor(shade),
                anti_alias: true,
                blend_mode: tiny_skia::BlendMode::Source,
                ..Default::default()
            };
            let stroke = Stroke {
                width: (radius * 2.0 * (1.0 - towards)).max(0.01),
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Default::default()
            };
            tube.stroke_path(&path, &paint, &stroke, into_tube, None);
        }

        pixmap.draw_pixmap(
            left.floor() as i32,
            top.floor() as i32,
            tube.as_ref(),
            &PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                quality: tiny_skia::FilterQuality::Nearest,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }

    fn draw_spinner(
        &self,
        pixmap: &mut Pixmap,
        object: &TimedObject,
        time_ms: f64,
        alpha: f32,
        layout: &Layout,
    ) {
        let progress =
            ((time_ms - object.start_ms) / object.duration_ms().max(1.0)).clamp(0.0, 1.0);
        let closing = SPINNER_RADIUS + (SPINNER_DOT - SPINNER_RADIUS) * progress;

        if self.skin_speaks_for(Element::SpinnerApproachCircle) {
            self.draw_sprite_wide(
                pixmap,
                Element::SpinnerApproachCircle,
                Point::CENTRE,
                layout.length(closing) * 2.0,
                alpha,
                layout,
            );
        } else {
            self.ring(
                pixmap,
                Point::CENTRE,
                layout.length(closing),
                layout.length(4.0),
                self.skin.spinner,
                alpha,
                layout,
            );
        }

        let old_style = self.spinner_is_old_style();
        if old_style {
            self.draw_spinner_layer(pixmap, Element::SpinnerBackground, alpha, layout);
            self.draw_spinner_metre(pixmap, object, time_ms, alpha, layout);
        } else {
            for layer in [Element::SpinnerBottom, Element::SpinnerGlow] {
                self.draw_spinner_layer(pixmap, layer, alpha, layout);
            }
        }

        let middle = self.spinner_middle();
        if let Some(middle) = middle {
            self.draw_spinner_layer(pixmap, middle, alpha, layout);
        }
        if !old_style {
            self.draw_spinner_layer_turned(
                pixmap,
                Element::SpinnerMiddle2,
                alpha,
                layout,
                self.spun_degrees(object, time_ms),
            );
            self.draw_spinner_layer(pixmap, Element::SpinnerTop, alpha, layout);
        }
        if middle.is_some() {
            self.draw_spin_bonus(pixmap, object, time_ms, alpha, layout);
            return;
        }

        let band = SPINNER_DOT - SPINNER_CORE;
        self.ring(
            pixmap,
            Point::CENTRE,
            layout.length(SPINNER_DOT - band / 2.0),
            layout.length(band),
            self.skin.spinner,
            alpha,
            layout,
        );
        self.dot(
            pixmap,
            Point::CENTRE,
            layout.length(SPINNER_CORE),
            lighten(self.skin.spinner, 0.55),
            alpha,
            layout,
        );

        self.draw_spin_bonus(pixmap, object, time_ms, alpha, layout);
    }

    fn own_width(&self, layout: &Layout, element: Element) -> f32 {
        self.skin
            .sprites
            .as_ref()
            .and_then(|s| s.get(element))
            .map_or(0.0, |sprite| self.skin_pixels(layout, sprite.width()))
    }

    fn draw_spinner_metre(
        &self,
        pixmap: &mut Pixmap,
        object: &TimedObject,
        time_ms: f64,
        alpha: f32,
        layout: &Layout,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };
        let Some((art, per)) = sprites.coloured(Element::SpinnerMetre, 0) else {
            return;
        };
        let required = dossier_sim::required_spins(self.state.difficulty(), object.duration_ms());
        if required <= 0.0 || alpha <= 0.0 {
            return;
        }
        let turned = dossier_sim::spinner_rotations(
            self.state.cursor_track(),
            object.start_ms,
            time_ms.min(object.end_ms),
        );
        let filled = ((turned / required) as f32).clamp(0.0, 1.0);
        if filled <= 0.0 {
            return;
        }

        let scale = layout.height as f32 / 768.0 / per;
        let (w, h) = (art.width() as f32 * scale, art.height() as f32 * scale);
        let shown = (h * filled).ceil().max(1.0) as u32;
        let Some(mut strip) = Pixmap::new(w.ceil().max(1.0) as u32, shown) else {
            return;
        };

        strip.draw_pixmap(
            0,
            0,
            art.as_ref(),
            &PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            Transform::from_translate(0.0, -(h - shown as f32)).pre_scale(scale, scale),
            None,
        );
        let (cx, cy) = layout.map(Point::CENTRE);
        pixmap.draw_pixmap(
            (cx - w / 2.0) as i32,
            (cy + h / 2.0 - shown as f32) as i32,
            strip.as_ref(),
            &PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }

    fn spinner_is_old_style(&self) -> bool {
        self.skin
            .sprites
            .as_ref()
            .is_some_and(|s| !s.draw_ourselves(Element::SpinnerBackground))
    }

    fn spinner_middle(&self) -> Option<Element> {
        let sprites = self.skin.sprites.as_ref()?;
        let wanted = if self.spinner_is_old_style() {
            Element::SpinnerCircle
        } else {
            Element::SpinnerMiddle
        };
        (!sprites.draw_ourselves(wanted)).then_some(wanted)
    }

    fn draw_spinner_layer(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        alpha: f32,
        layout: &Layout,
    ) {
        self.draw_spinner_layer_turned(pixmap, element, alpha, layout, 0.0);
    }

    fn draw_spinner_layer_turned(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        alpha: f32,
        layout: &Layout,
        degrees: f32,
    ) {
        let own = self.own_width(layout, element);
        if own > 0.0 {
            self.draw_wide(
                pixmap,
                element,
                Point::CENTRE,
                own,
                alpha,
                layout,
                degrees,
                0,
            );
        }
    }

    fn follow_pulse(&self, index: usize, time_ms: f64) -> f32 {
        let Some(judge) = self.state.judge() else {
            return 1.0;
        };
        let mut newest = f64::NEG_INFINITY;
        for event in judge.events_for(index) {
            if !matches!(
                event.part,
                dossier_sim::Part::SliderTick | dossier_sim::Part::SliderRepeat
            ) {
                continue;
            }
            if event.result == dossier_sim::Judgement::Miss || event.time_ms > time_ms {
                continue;
            }
            newest = newest.max(event.time_ms);
        }
        if !newest.is_finite() {
            return 1.0;
        }
        let age = (time_ms - newest) / FOLLOW_BEAT_MS;
        if !(0.0..1.0).contains(&age) {
            return 1.0;
        }

        let fade = 1.0 - age;
        1.0 + FOLLOW_BEAT * (fade * fade) as f32
    }

    fn spun_degrees(&self, object: &TimedObject, time_ms: f64) -> f32 {
        let facing = dossier_sim::spinner_facing(
            self.state.cursor_track(),
            object.start_ms,
            time_ms.min(object.end_ms),
        );
        (facing * 360.0) as f32
    }

    fn draw_spin_bonus(
        &self,
        pixmap: &mut Pixmap,
        object: &TimedObject,
        time_ms: f64,
        alpha: f32,
        layout: &Layout,
    ) {
        let Some(font) = self.skin.font.as_ref() else {
            return;
        };
        let Some(judge) = self.state.judge() else {
            return;
        };

        let mut awarded = 0u32;
        let mut latest = f64::NEG_INFINITY;
        for event in judge.events() {
            if event.part != dossier_sim::Part::SpinnerBonus || event.time_ms > time_ms {
                continue;
            }
            if event.time_ms < object.start_ms || event.time_ms > object.end_ms {
                continue;
            }
            awarded += 1;
            latest = latest.max(event.time_ms);
        }
        if awarded == 0 {
            return;
        }

        let age = time_ms - latest;

        let flash = (1.0 - (age / SPINNER_BONUS_PULSE_MS).clamp(0.0, 1.0)) as f32;
        let eased = flash * flash * flash;
        let size = layout.length(SPINNER_BONUS_SIZE) * (1.0 + SPINNER_BONUS_SWELL * eased);

        let colour = lighten(darken(self.skin.spinner, SPINNER_BONUS_REST), eased);
        let at = layout.map(Point {
            x: Point::CENTRE.x,
            y: Point::CENTRE.y + SPINNER_BONUS_BELOW,
        });

        let text = format!("{}", awarded * SPINNER_BONUS_STEP);
        if self.draw_hud_text(
            pixmap,
            &text,
            at.0,
            at.1 + size / 2.0,
            size,
            Align::Centre,
            alpha,
        ) {
            return;
        }
        font.draw(
            pixmap,
            Label {
                text: &text,
                x: at.0,
                y: at.1 + size * 0.35,
                size,
                colour: with_alpha(colour, alpha),
                align: Align::Centre,
            },
        );
    }

    fn draw_trail(&self, pixmap: &mut Pixmap, time_ms: f64, radius: f32, layout: &Layout) {
        let track = self.state.cursor_track();

        let disjoint = !self.skin_speaks_for(Element::CursorMiddle);

        let skinned = self.skin_speaks_for(Element::CursorTrail);
        let own = self
            .skin
            .sprites
            .as_ref()
            .and_then(|s| s.get(Element::CursorTrail))
            .map_or(radius * 2.0, |sprite| {
                self.skin_pixels(layout, sprite.width()) * self.skin.cursor_scale
            });

        let mut mark = |at: dossier_beatmap::Point, alpha: f32| {
            if alpha <= 0.0 {
                return;
            }
            if skinned {
                self.draw_sprite_wide(pixmap, Element::CursorTrail, at, own, alpha, layout);
            } else {
                self.dot(
                    pixmap,
                    at,
                    radius * 0.8,
                    self.skin.trail_colour,
                    alpha,
                    layout,
                );
            }
        };

        if disjoint {
            let mut age = TRAIL_STEP_MS;
            while age <= TRAIL_DISJOINT_MS {
                if let Some(sample) = track.sample(time_ms - age) {
                    mark(sample.pos, 1.0 - (age / TRAIL_DISJOINT_MS) as f32);
                }
                age += TRAIL_STEP_MS;
            }
            return;
        }

        let interval = (own * TRAIL_INTERVAL_SHARE / layout.length(1.0).max(0.001)) as f64;
        if interval <= 0.0 {
            return;
        }
        let Some(head) = track.sample(time_ms) else {
            return;
        };
        let (mut last, mut walked) = (head.pos, 0.0f64);
        let mut age = 0.0f64;
        while age < TRAIL_CONTINUOUS_MS {
            age += TRAIL_STEP_MS / 4.0;
            let Some(sample) = track.sample(time_ms - age) else {
                break;
            };
            let step = f64::from((sample.pos.x - last.x).hypot(sample.pos.y - last.y));
            last = sample.pos;
            walked += step;
            if walked < interval {
                continue;
            }
            walked = 0.0;
            mark(sample.pos, 1.0 - (age / TRAIL_CONTINUOUS_MS) as f32);
        }
    }

    fn cursor_turn(&self, time_ms: f64) -> f32 {
        let allowed = self.skin.cursor_rotate.unwrap_or_else(|| {
            self.skin
                .sprites
                .as_ref()
                .is_none_or(|sprites| sprites.ini().cursor_rotate)
        });
        if !allowed {
            return 0.0;
        }

        (time_ms.rem_euclid(CURSOR_TURN_MS) / CURSOR_TURN_MS * 360.0) as f32
    }

    pub(super) fn draw_cursor(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let track = self.state.cursor_track();
        let radius = layout.length(9.0);

        if self.skin.cursor_trail {
            self.draw_trail(pixmap, time_ms, radius, layout);
        }

        if let Some(sample) = track.sample(time_ms) {
            let expands = self.skin.cursor_expand
                && self
                    .skin
                    .sprites
                    .as_ref()
                    .is_none_or(|sprites| sprites.ini().cursor_expand);
            let held = expands && sample.keys.is_pressed();
            if self.skin_speaks_for(Element::Cursor) {
                let own = self
                    .skin
                    .sprites
                    .as_ref()
                    .and_then(|s| s.get(Element::Cursor))
                    .map_or(radius * 2.0 * self.skin.cursor_scale, |sprite| {
                        self.skin_pixels(layout, sprite.width()) * self.skin.cursor_scale
                    });
                let wide = own * if held { 1.25 } else { 1.0 };
                self.draw_sprite_wide_turned(
                    pixmap,
                    Element::Cursor,
                    sample.pos,
                    wide,
                    1.0,
                    layout,
                    self.cursor_turn(time_ms),
                );
                if self.skin_speaks_for(Element::CursorMiddle) {
                    let middle = self
                        .skin
                        .sprites
                        .as_ref()
                        .and_then(|s| s.get(Element::CursorMiddle))
                        .map_or(radius * 2.0, |sprite| {
                            self.skin_pixels(layout, sprite.width()) * self.skin.cursor_scale
                        });
                    self.draw_sprite_wide(
                        pixmap,
                        Element::CursorMiddle,
                        sample.pos,
                        middle,
                        1.0,
                        layout,
                    );
                }
                return;
            }

            let scale = self.skin.cursor_scale;
            self.dot(
                pixmap,
                sample.pos,
                radius * 1.25 * scale,
                self.skin.trail_colour,
                0.5,
                layout,
            );
            self.dot(
                pixmap,
                sample.pos,
                radius * if held { 0.95 } else { 0.75 } * scale,
                self.skin.cursor,
                1.0,
                layout,
            );
        }
    }

    fn dot(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        colour: tiny_skia::Color,
        alpha: f32,
        layout: &Layout,
    ) {
        let (x, y) = layout.map(centre);
        crate::elements::dot(pixmap, x, y, radius, colour, alpha);
    }

    fn glow(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        colour: tiny_skia::Color,
        alpha: f32,
        layout: &Layout,
    ) {
        let (x, y) = layout.map(centre);
        crate::elements::glow(pixmap, x, y, radius, colour, alpha, self.skin.note_glow);
    }

    fn lit_dot(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        colour: tiny_skia::Color,
        alpha: f32,
        layout: &Layout,
    ) {
        let (x, y) = layout.map(centre);
        crate::elements::lit_dot(pixmap, x, y, radius, colour, alpha, self.skin.note_relief);
    }

    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
        pixmap: &mut Pixmap,
        centre: Point,
        radius: f32,
        width: f32,
        colour: tiny_skia::Color,
        alpha: f32,
        layout: &Layout,
    ) {
        let (x, y) = layout.map(centre);
        crate::elements::ring(pixmap, x, y, radius, width, colour, alpha);
    }
}

fn shaken(pos: Point, annotation: &Annotation, time_ms: f64, state: &GameState) -> Point {
    let radius = state.difficulty().circle_radius();
    let dx = shake_offset(&annotation.shakes_ms, time_ms, radius);
    Point {
        x: pos.x + dx,
        y: pos.y,
    }
}

fn shake_offset(shakes: &[f64], time_ms: f64, radius: f64) -> f64 {
    let Some(last) = shakes
        .iter()
        .copied()
        .filter(|&at| at <= time_ms && time_ms - at < SHAKE_MS)
        .fold(None::<f64>, |best, at| {
            Some(best.map_or(at, |b: f64| b.max(at)))
        })
    else {
        return 0.0;
    };
    let progress = (time_ms - last) / SHAKE_MS;
    let swing = (progress * SHAKE_CYCLES * std::f64::consts::TAU).sin();
    swing * (1.0 - progress) * radius * SHAKE_WIDTH
}

fn arrow_rock(time_ms: f64, started_ms: f64) -> f32 {
    const ROTATION: f32 = 5.625;
    let phase = ((time_ms - started_ms).rem_euclid(ARROW_LOOP_MS) / ARROW_LOOP_MS) as f32;

    ROTATION - 2.0 * ROTATION * phase
}

fn arrow_life(
    turns: &[(f64, f64)],
    time_ms: f64,
    reading_ms: f64,
    started_ms: f64,
    span_ms: f64,
) -> (f32, f32) {
    let ahead = turns
        .iter()
        .any(|&(at, due)| at > time_ms && reading_ms >= due);
    let behind = turns
        .iter()
        .map(|&(at, _)| at)
        .filter(|&at| at <= time_ms)
        .fold(None::<f64>, |best, at| {
            Some(best.map_or(at, |b: f64| b.max(at)))
        });

    let arriving = turns
        .iter()
        .filter(|&&(at, due)| at > time_ms && reading_ms >= due)
        .map(|&(_, due)| {
            if due <= started_ms {
                1.0
            } else {
                ((reading_ms - due) / ARROW_FADE_MS).clamp(0.0, 1.0) as f32
            }
        })
        .fold(0.0f32, f32::max);

    let leaving = match (ahead, behind) {
        (true, _) => arriving,
        (false, Some(last)) => 1.0 - ((time_ms - last) / ARROW_FADE_MS).clamp(0.0, 1.0) as f32,
        (false, None) => 0.0,
    };

    let ease = |t: f32| 1.0 - (1.0 - t.clamp(0.0, 1.0)) * (1.0 - t.clamp(0.0, 1.0));
    let scale = match behind {
        Some(last) => {
            let over = span_ms.min(ARROW_LOOP_MS).max(1.0);
            1.0 + (ARROW_STRUCK_TO - 1.0) * ease(((time_ms - last) / over) as f32)
        }

        None => {
            let phase = ((time_ms - started_ms).rem_euclid(ARROW_LOOP_MS) / ARROW_LOOP_MS) as f32;
            ARROW_LOOP_FROM + (1.0 - ARROW_LOOP_FROM) * ease(phase)
        }
    };
    (leaving, scale)
}

fn path_fraction(object: &TimedObject, time_ms: f64) -> Option<f64> {
    let TimedKind::Slider {
        slides,
        slide_duration_ms,
        ..
    } = &object.kind
    else {
        return None;
    };
    if *slide_duration_ms <= 0.0 {
        return None;
    }
    let travelled = (time_ms - object.start_ms) / slide_duration_ms;
    let last = f64::from(slides.saturating_sub(1));
    let slide = travelled.floor().clamp(0.0, last);
    let local = (travelled - slide).clamp(0.0, 1.0);
    Some(if (slide as u32).is_multiple_of(2) {
        local
    } else {
        1.0 - local
    })
}

fn hit_expansion(exit: f32, missed: bool) -> f32 {
    if missed {
        1.0
    } else {
        1.0 + 0.4 * (1.0 - (1.0 - exit) * (1.0 - exit))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenFade {
    Own,

    AsANote,

    Untouched,
}

fn body_path(object: &TimedObject, (from, to): (f64, f64)) -> Option<tiny_skia::Path> {
    let TimedKind::Slider { path, .. } = &object.kind else {
        return None;
    };
    let (start, interior, end) = path.segment(from, to)?;

    let mut builder = PathBuilder::with_capacity(interior.len() + 2, interior.len() + 2);
    builder.move_to(start.x as f32, start.y as f32);
    for point in interior {
        builder.line_to(point.x as f32, point.y as f32);
    }
    builder.line_to(end.x as f32, end.y as f32);
    builder.finish()
}

#[cfg(test)]
mod exits {
    use super::*;

    const SPAN: f64 = 2000.0;

    fn turn(at: f64) -> (f64, f64) {
        (at, at - SPAN)
    }

    #[test]
    fn an_arrow_waits_until_the_ball_sets_off_towards_it() {
        let turns = [turn(5000.0)];
        assert_eq!(
            arrow_life(&turns, 2000.0, 2000.0, 0.0, 500.0).0,
            0.0,
            "two traversals out, nothing there yet"
        );

        assert_eq!(
            arrow_life(&turns, 3000.0, 3000.0, 2500.0, 500.0).0,
            0.0,
            "one traversal out, to the millisecond: it begins arriving"
        );
        let midway = arrow_life(
            &turns,
            3000.0 + ARROW_FADE_MS * 0.5,
            3000.0 + ARROW_FADE_MS * 0.5,
            2500.0,
            500.0,
        )
        .0;
        assert!(
            (0.3..0.7).contains(&midway),
            "halfway through arriving: {midway}"
        );
        assert_eq!(
            arrow_life(
                &turns,
                3000.0 + ARROW_FADE_MS,
                3000.0 + ARROW_FADE_MS,
                2500.0,
                500.0
            )
            .0,
            1.0,
            "and fully there once its fade is done"
        );
    }

    #[test]
    fn an_arrow_holds_while_a_turn_is_coming_and_then_goes_out() {
        let turns = [turn(1000.0), turn(3000.0)];
        assert_eq!(
            arrow_life(&turns, 500.0, 500.0, 0.0, 500.0).0,
            1.0,
            "before the first"
        );
        assert_eq!(
            arrow_life(&turns, 2500.0, 2500.0, 0.0, 500.0).0,
            1.0,
            "another is still coming, and has finished arriving"
        );

        let half = arrow_life(
            &turns,
            3000.0 + ARROW_FADE_MS / 2.0,
            3000.0 + ARROW_FADE_MS / 2.0,
            0.0,
            500.0,
        )
        .0;
        assert!(half > 0.0 && half < 1.0, "{half}");
        assert_eq!(
            arrow_life(
                &turns,
                3000.0 + ARROW_FADE_MS,
                3000.0 + ARROW_FADE_MS,
                2500.0,
                500.0
            )
            .0,
            0.0,
            "and is gone"
        );
    }

    #[test]
    fn an_arrow_waiting_breathes_on_a_fixed_loop() {
        let turns = [turn(4000.0)];
        let at = |t: f64| arrow_life(&turns, t, t, 0.0, 500.0).1;
        assert!(
            (at(0.0) - ARROW_LOOP_FROM).abs() < 1e-6,
            "largest at the start"
        );
        assert!(at(ARROW_LOOP_MS - 1.0) < 1.02, "and smallest at the end");

        assert!((at(ARROW_LOOP_MS) - ARROW_LOOP_FROM).abs() < 1e-6);
        assert!((at(ARROW_LOOP_MS * 3.0) - ARROW_LOOP_FROM).abs() < 1e-6);
    }

    #[test]
    fn a_struck_arrow_grows_into_the_turn_it_marked() {
        let turns = [turn(1000.0)];
        let at = |t: f64, span: f64| arrow_life(&turns, t, t, 0.0, span).1;
        assert!(
            (at(1000.0, 500.0) - 1.0).abs() < 1e-6,
            "starts at its own size"
        );
        assert!(
            (at(1300.0, 500.0) - ARROW_STRUCK_TO).abs() < 1e-6,
            "and reaches 1.4"
        );

        assert!((at(1120.0, 120.0) - ARROW_STRUCK_TO).abs() < 1e-6);
        assert!(
            at(1060.0, 120.0) < ARROW_STRUCK_TO,
            "part-way at half the span"
        );
    }

    #[test]
    fn an_old_skins_arrow_leans_one_way_and_then_the_other() {
        assert!(
            (arrow_rock(1000.0, 1000.0) - 5.625).abs() < 1e-4,
            "right, at the top"
        );
        assert!(arrow_rock(1150.0, 1000.0).abs() < 1e-4, "level, halfway");
        assert!(arrow_rock(1290.0, 1000.0) < -5.0, "and left by the end");

        assert!(
            (arrow_rock(1300.0, 1000.0) - arrow_rock(1000.0, 1000.0)).abs() < 1e-4,
            "the loop does not drift"
        );
    }

    #[test]
    fn an_end_that_never_turns_shows_nothing() {
        assert_eq!(arrow_life(&[], 1234.0, 1234.0, 0.0, 500.0).0, 0.0);
    }

    #[test]
    fn a_hit_swells_as_it_goes_and_a_miss_does_not() {
        assert_eq!(hit_expansion(0.0, false), 1.0, "nothing has happened yet");
        assert!(hit_expansion(1.0, false) > hit_expansion(0.5, false));
        assert_eq!(hit_expansion(1.0, true), 1.0, "a miss keeps its size");
        assert_eq!(hit_expansion(0.5, true), 1.0);
    }
}

#[cfg(test)]
mod cost {
    use super::*;

    #[test]
    #[ignore = "a measurement, not an assertion"]
    fn path_building_against_stroking() {
        use std::time::Instant;

        let points: Vec<(f32, f32)> = (0..240)
            .map(|i| (i as f32 * 1.7, (i as f32 * 0.11).sin() * 40.0 + 200.0))
            .collect();

        let rounds = 10_000;
        let mark = Instant::now();
        let mut kept = 0usize;
        for _ in 0..rounds {
            let mut builder = PathBuilder::with_capacity(points.len(), points.len());
            builder.move_to(points[0].0, points[0].1);
            for p in &points[1..] {
                builder.line_to(p.0, p.1);
            }
            kept += builder.finish().map_or(0, |p| p.len());
        }
        let building = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

        let mut pixmap = Pixmap::new(1920, 1080).unwrap();
        let mut builder = PathBuilder::with_capacity(points.len(), points.len());
        builder.move_to(points[0].0, points[0].1);
        for p in &points[1..] {
            builder.line_to(p.0, p.1);
        }
        let path = builder.finish().unwrap();
        let paint = Paint::default();
        let stroke = Stroke {
            width: 64.0,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Default::default()
        };

        let strokes = 200;
        let mark = Instant::now();
        for _ in 0..strokes {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
        let stroking = mark.elapsed().as_secs_f64() / f64::from(strokes) * 1000.0;

        println!(
            "slider body: building {building:.4}ms, stroking {stroking:.4}ms \
             — building is {:.2}% of one stroke ({kept} verbs kept)",
            building / stroking * 100.0
        );
    }
}

const FOLLOW_BEAT: f32 = 0.10;
const FOLLOW_BEAT_MS: f64 = 110.0;

const FOLLOW_LEAVE_MS: f64 = 200.0;

const FOLLOW_LEAVE_TO: f32 = 0.8;

const FOLLOW_SPACING: f64 = 32.0;
const FOLLOW_PREEMPT_MS: f64 = 800.0;

const FOLLOW_ENTRY_SCALE: f32 = 1.5;

const FOLLOW_APPROACH: f64 = 0.1;

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

impl Scene<'_> {
    pub(super) fn draw_follow_points(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        if !self.skin_speaks_for(Element::FollowPoint) {
            return;
        }
        let objects = &self.state.timeline().objects;
        let fade_in = self.state.difficulty().fade_in_ms().max(1.0);
        let radius = self.state.difficulty().circle_radius();

        for index in self.candidates(time_ms) {
            if index == 0 {
                continue;
            }
            let (from, to) = (&objects[index - 1], &objects[index]);

            if self.annotations[index].number == 1 || from.is_spinner() || to.is_spinner() {
                continue;
            }
            let start_ms = from.end_ms;
            let span = to.start_ms - start_ms;
            if span <= 0.0 {
                continue;
            }

            let leaves = from.ball_at(from.end_ms).unwrap_or(from.pos);
            let (dx, dy) = (to.pos.x - leaves.x, to.pos.y - leaves.y);
            let distance = dx.hypot(dy);
            if distance <= FOLLOW_SPACING * 2.5 {
                continue;
            }

            let degrees = dy.atan2(dx).to_degrees() as f32;

            let mut walked = FOLLOW_SPACING * 1.5;
            while walked < distance - FOLLOW_SPACING {
                let fraction = walked / distance;
                walked += FOLLOW_SPACING;

                let leaves_at = start_ms + fraction * span;
                let arrives_at = leaves_at - FOLLOW_PREEMPT_MS;
                if time_ms < arrives_at {
                    continue;
                }
                let arriving = ((time_ms - arrives_at) / fade_in).clamp(0.0, 1.0) as f32;
                let leaving = if time_ms > leaves_at {
                    ((time_ms - leaves_at) / fade_in).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                let alpha = arriving * (1.0 - leaving);
                if alpha <= 0.0 {
                    continue;
                }

                let along = fraction - FOLLOW_APPROACH * f64::from(1.0 - ease_out(arriving));
                let at = dossier_beatmap::Point {
                    x: leaves.x + dx * along,
                    y: leaves.y + dy * along,
                };
                let scale = FOLLOW_ENTRY_SCALE + (1.0 - FOLLOW_ENTRY_SCALE) * ease_out(arriving);
                self.draw_frame_turned(
                    pixmap,
                    Element::FollowPoint,
                    self.animation_frame(Element::FollowPoint, time_ms - arrives_at),
                    at,
                    layout.length(radius) * scale,
                    alpha,
                    layout,
                    degrees,
                );
            }
        }
    }
}

impl Scene<'_> {
    fn animation_frame(&self, element: Element, elapsed_ms: f64) -> usize {
        self.frame_of(element, elapsed_ms, true)
    }

    fn animation_frame_once(&self, element: Element, elapsed_ms: f64) -> usize {
        self.frame_of(element, elapsed_ms, false)
    }

    fn frame_of(&self, element: Element, elapsed_ms: f64, looping: bool) -> usize {
        let Some(sprites) = &self.skin.sprites else {
            return 0;
        };
        let count = sprites.frame_count(element);
        if count <= 1 {
            return 0;
        }
        let stated = sprites.ini().animation_framerate;
        let per_second = if stated > 0.0 {
            f64::from(stated)
        } else {
            count as f64
        };
        let at = (elapsed_ms.max(0.0) / 1000.0 * per_second) as usize;
        if looping {
            at % count
        } else {
            at.min(count - 1)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_frame_turned(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        frame: usize,
        centre: Point,
        radius: f32,
        alpha: f32,
        layout: &Layout,
        degrees: f32,
    ) {
        let art = self
            .skin
            .sprites
            .as_ref()
            .and_then(|sprites| sprites.frame(element, frame));
        let Some((art, per_osu_pixel)) = art else {
            self.draw_sprite_turned(pixmap, element, 0, centre, radius, alpha, layout, degrees);
            return;
        };
        if alpha <= 0.0 {
            return;
        }
        let scale = (radius * 2.0) / (SKIN_CIRCLE_PIXELS * per_osu_pixel);
        let (x, y) = layout.map(centre);
        let transform = Transform::from_translate(x, y)
            .pre_rotate(degrees)
            .pre_scale(scale, scale)
            .pre_translate(-(art.width() as f32) / 2.0, -(art.height() as f32) / 2.0);
        pixmap.draw_pixmap(
            0,
            0,
            art.as_ref(),
            &PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            transform,
            None,
        );
    }
}

#[cfg(test)]
mod shading {
    use super::*;

    fn track() -> Color {
        Color::from_rgba8(0, 120, 255, 255)
    }

    fn shade(at: f32) -> Color {
        tube_shade(
            at,
            Color::from_rgba8(255, 255, 255, 255),
            crate::skin::body_outer(track()),
            crate::skin::body_inner(track()),
            0.7,
        )
    }

    #[test]
    fn the_outermost_sliver_is_a_shadow_coming_up_from_nothing() {
        assert!(shade(0.0).alpha() < 0.001, "nothing at the very edge");
        let inner = shade(SHADOW_PORTION);
        assert!(
            (inner.alpha() - SHADOW_ALPHA).abs() < 0.01,
            "{}",
            inner.alpha()
        );
        assert!(
            inner.red() + inner.green() + inner.blue() < 0.01,
            "and it is black"
        );

        assert!((shade(SHADOW_PORTION / 2.0).alpha() - SHADOW_ALPHA / 2.0).abs() < 0.01);
    }

    #[test]
    fn the_border_is_one_colour_across_its_whole_width() {
        let white = Color::from_rgba8(255, 255, 255, 255);
        for at in [SHADOW_PORTION + 0.001, 0.12, BORDER_PORTION] {
            let there = shade(at);
            assert!(
                (there.red() - white.red()).abs() < 0.001
                    && (there.alpha() - white.alpha()).abs() < 0.001,
                "the border is not solid at {at}: {there:?}"
            );
        }
    }

    #[test]
    fn the_body_mixes_straight_from_the_border_to_the_centreline() {
        let outer = crate::skin::body_outer(track());
        let inner = crate::skin::body_inner(track());
        let at_start = shade(BORDER_PORTION + 0.0001);
        assert!(
            (at_start.green() - outer.green()).abs() < 0.01,
            "starts at the outer shade"
        );
        let at_end = shade(1.0);
        assert!(
            (at_end.green() - inner.green()).abs() < 0.01,
            "ends at the inner one"
        );

        let half = shade(BORDER_PORTION + (1.0 - BORDER_PORTION) / 2.0);
        let expect = (outer.green() + inner.green()) / 2.0;
        assert!(
            (half.green() - expect).abs() < 0.01,
            "{} against {expect}",
            half.green()
        );
    }

    #[test]
    fn the_track_carries_the_alpha_the_game_gives_it() {
        assert!((shade(0.5).alpha() - 0.7).abs() < 0.001);
        assert!((shade(1.0).alpha() - 0.7).abs() < 0.001);
    }
}
