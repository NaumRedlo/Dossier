use super::*;

use tiny_skia::{Color, Paint, Pixmap, Transform};

use crate::layout::Layout;
use crate::skin::{with_alpha, ArrowShape};

impl Scene<'_> {
    pub(super) fn compose_fail(
        &self,
        out: &mut Pixmap,
        field: &Pixmap,
        overlay: &Pixmap,
        progress: f32,
        presence: f32,
        layout: &Layout,
    ) {
        self.ground(out);

        let scale = if progress <= FAIL_RELEASE_AT {
            let t = progress / FAIL_RELEASE_AT;
            1.0 - (1.0 - FAIL_SQUEEZE) * (1.0 - (1.0 - t).powi(3))
        } else {
            let t = (progress - FAIL_RELEASE_AT) / (1.0 - FAIL_RELEASE_AT);
            FAIL_SQUEEZE + (1.0 - FAIL_SQUEEZE) * (1.0 - (1.0 - t).powi(3))
        };

        let (cx, cy) = (layout.width as f32 / 2.0, layout.height as f32 / 2.0);
        let transform = Transform::from_translate(cx, cy)
            .pre_scale(scale, scale)
            .pre_translate(-cx, -cy);

        let blit = |out: &mut Pixmap, src: &Pixmap, opacity: f32| {
            let paint = tiny_skia::PixmapPaint {
                opacity,
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            };
            out.draw_pixmap(0, 0, src.as_ref(), &paint, transform, None);
        };

        blit(out, field, presence);
        blit(out, overlay, presence);

        let wash = |out: &mut Pixmap, colour: Color, blend: tiny_skia::BlendMode| {
            let mut paint = Paint::default();
            paint.set_color(colour);
            paint.anti_alias = false;
            paint.blend_mode = blend;
            if let Some(rect) = Rect::from_xywh(0.0, 0.0, layout.width as f32, layout.height as f32)
            {
                out.fill_rect(rect, &paint, Transform::identity(), None);
            }
        };

        let linear =
            1.0 - (progress * FAIL_ANIMATION_MS as f32 / FAIL_FLASH_MS as f32).clamp(0.0, 1.0);
        let flash = linear * linear;
        if flash > 0.0 {
            wash(
                out,
                with_alpha(self.skin.verdict_miss, flash * FAIL_FLASH_ALPHA),
                tiny_skia::BlendMode::Plus,
            );
        }
    }

    pub(super) fn draw_danger(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        if self.cannot_die() {
            return;
        }
        let Some(health) = self.state.health_at(time_ms) else {
            return;
        };
        if health >= DANGER_FROM {
            return;
        }

        let closeness = ((DANGER_FROM - health) / DANGER_FROM).clamp(0.0, 1.0);
        let strength = closeness * closeness * DANGER_MAX;

        let (w, h) = (layout.width as f32, layout.height as f32);
        let reach = h * DANGER_REACH;

        for step in 0..DANGER_BANDS {
            let t = step as f32 / DANGER_BANDS as f32;
            let alpha = strength * (1.0 - t) * (1.0 - t) / DANGER_BANDS as f32 * 3.0;
            let colour = with_alpha(self.skin.verdict_miss, alpha);
            let band = reach / DANGER_BANDS as f32;
            let inset = t * reach;
            for rect in [
                Rect::from_xywh(0.0, inset, w, band),
                Rect::from_xywh(0.0, h - inset - band, w, band),
                Rect::from_xywh(inset, 0.0, band, h),
                Rect::from_xywh(w - inset - band, 0.0, band, h),
            ]
            .into_iter()
            .flatten()
            {
                let mut paint = Paint::default();
                paint.set_color(colour);
                paint.anti_alias = false;
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
    }

    pub(super) fn beat_kick(&self, time_ms: f64) -> f32 {
        let Some(point) = self.state.timeline().timing.timing_point_at(time_ms) else {
            return 0.0;
        };
        if point.beat_length <= 0.0 {
            return 0.0;
        }
        let phase = ((time_ms - point.time_ms) / point.beat_length).rem_euclid(1.0) as f32;
        (1.0 - phase) * (1.0 - phase)
    }

    pub(super) fn draw_verdicts(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let radius = self.state.difficulty().circle_radius();

        for index in self.candidates(time_ms) {
            let annotation = &self.annotations[index];
            let Some(verdict) = annotation.verdict else {
                continue;
            };
            let age = time_ms - annotation.resolved_ms;
            if !(0.0..VERDICT_MS).contains(&age) {
                continue;
            }
            if verdict == Judgement::Great && !self.skin.show_300 {
                continue;
            }

            let element = crate::elements::Element::Verdict(match verdict {
                Judgement::Great => crate::elements::Verdict::Three,
                Judgement::Ok => crate::elements::Verdict::Hundred,
                Judgement::Meh => crate::elements::Verdict::Fifty,
                Judgement::Miss => crate::elements::Verdict::Miss,
            });
            let alpha = verdict_alpha(age);

            let (text, colour) = match verdict {
                Judgement::Great => ("300", self.skin.verdict_300),
                Judgement::Ok => ("100", self.skin.verdict_100),
                Judgement::Meh => ("50", self.skin.verdict_50),
                Judgement::Miss => ("×", self.skin.verdict_miss),
            };
            let scale = VERDICT_TEXT_SCALE;

            let presence = match verdict {
                Judgement::Great => 0.70,
                Judgement::Ok => 0.85,
                Judgement::Meh => 0.92,
                Judgement::Miss => 1.0,
            };

            let object = &self.state.timeline().objects[index];
            let at_head = annotation.judged_before_the_end(object);
            let mut at = layout.map(verdict_place(object, at_head));

            if verdict == Judgement::Miss && self.skin_version() > 1.0 {
                at.1 += layout.length(miss_drift(age));
            }
            let animated = self.skin.sprites.as_ref().is_some_and(|sprites| sprites.animated(element));
            let settle = if animated { 1.0 } else { verdict_settle(age, verdict == Judgement::Miss) };
            let size = layout.length(radius * scale) * settle;
            if self.skin_speaks_for(element) {
                let own = self
                    .skin
                    .sprites
                    .as_ref()
                    .and_then(|sprites| Some((sprites, sprites.get(element)?)))
                    .map_or(0.0, |(sprites, sprite)| {
                        let full = layout.length(f64::from(sprite.width()));

                        full * verdict_held(sprites, element, radius) as f32
                    });

                if own > 0.0 {
                    self.draw_sprite_wide_at(
                        pixmap,
                        element,
                        verdict_place(object, at_head),
                        own * settle,
                        alpha * presence,
                        layout,
                        age,
                    );
                }
                continue;
            }
            let Some(font) = &self.skin.font else {
                continue;
            };
            font.draw(
                pixmap,
                Label {
                    text,
                    x: at.0,
                    y: at.1 + size * 0.35,
                    size,
                    colour: with_alpha(colour, alpha * presence),
                    align: Align::Centre,
                },
            );
        }
    }

    pub(super) fn draw_section(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let Some(&(from, to)) = self
            .state
            .timeline()
            .breaks
            .iter()
            .find(|&&(from, to)| time_ms >= from && time_ms <= to)
        else {
            return;
        };
        let length = to - from;
        if length < SECTION_MIN_BREAK_MS {
            return;
        }
        let at = (to - SECTION_MIN_BREAK_MS).min(to - length / 2.0);

        let passing = self
            .state
            .health_at(at)
            .is_none_or(|health| health >= SECTION_PASS_HEALTH);
        let element = if passing {
            crate::elements::Element::SectionPass
        } else {
            crate::elements::Element::SectionFail
        };
        if !self.skin_speaks_for(element) {
            return;
        }

        let steps: &[(f64, f32)] = if passing {
            &[
                (20.0, 1.0),
                (100.0, 0.0),
                (160.0, 1.0),
                (230.0, 0.0),
                (280.0, 1.0),
            ]
        } else {
            &[(130.0, 1.0), (230.0, 0.0), (280.0, 1.0)]
        };
        let since = time_ms - at;
        if since < steps[0].0 {
            return;
        }
        let mut alpha = 0.0;
        for &(offset, level) in steps {
            if since >= offset {
                alpha = level;
            }
        }
        if since >= SECTION_FADE_FROM_MS {
            let out = ((since - SECTION_FADE_FROM_MS) / (SECTION_FADE_TO_MS - SECTION_FADE_FROM_MS))
                .clamp(0.0, 1.0) as f32;
            alpha = 1.0 - out;
        }
        if alpha <= 0.0 {
            return;
        }

        let own = self
            .skin
            .sprites
            .as_ref()
            .and_then(|sprites| sprites.get(element))
            .map_or(0.0, |sprite| self.skin_pixels(layout, sprite.width()));
        if own <= 0.0 {
            return;
        }
        self.draw_sprite_wide(
            pixmap,
            element,
            dossier_beatmap::Point::CENTRE,
            own,
            alpha,
            layout,
        );
    }

    pub(super) fn draw_break_warning(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let Some(ends) = self
            .state
            .timeline()
            .breaks
            .iter()
            .find(|(starts, ends)| time_ms >= *starts && time_ms < *ends + WARNING_EXIT_MS)
            .map(|&(_, ends)| ends)
        else {
            return;
        };

        let (alpha, scale) = if time_ms < ends {
            let left = ends - time_ms;
            if left > WARNING_MS {
                return;
            }

            let entering = ((WARNING_MS - left) / WARNING_ENTRY_MS).clamp(0.0, 1.0) as f32;
            let kick = self.beat_kick(time_ms);

            (
                (WARNING_REST + WARNING_BEAT * kick) * entering,
                1.0 + WARNING_SWELL * kick,
            )
        } else {
            let leaving = ((time_ms - ends) / WARNING_EXIT_MS).clamp(0.0, 1.0);
            let left = 1.0 - leaving;
            ((left * left) as f32, (1.0 - 0.45 * leaving) as f32)
        };
        if alpha <= 0.01 {
            return;
        }

        let arrow = self.state.difficulty().circle_radius() * WARNING_SIZE;

        let reach = arrow * (1.0 + f64::from(ARROW_ROUNDING) / 2.0);
        let size = layout.length(arrow) * scale;
        let skinned = self.skin_speaks_for(crate::elements::Element::WarningArrow);
        for y in WARNING_ROWS {
            for (x, dir) in [
                (-reach, (1.0, 0.0)),
                (dossier_beatmap::PLAYFIELD_WIDTH + reach, (-1.0, 0.0)),
            ] {
                let at = Point { x, y };
                if skinned {
                    self.draw_warning_arrow(pixmap, at, dir.0 < 0.0, size, alpha, layout);
                    continue;
                }
                self.draw_chevron(
                    pixmap,
                    Turn { at, dir },
                    size,
                    alpha,
                    ArrowShape::Rounded,
                    layout,
                );
            }
        }
    }

    fn draw_warning_arrow(
        &self,
        pixmap: &mut Pixmap,
        at: Point,
        facing_left: bool,
        size: f32,
        alpha: f32,
        layout: &Layout,
    ) {
        let element = crate::elements::Element::WarningArrow;
        if facing_left {
            self.draw_sprite_mirrored(pixmap, element, 0, at, size, alpha, layout);
        } else {
            self.draw_sprite(pixmap, element, 0, at, size, alpha, layout);
        }
    }
}

fn verdict_held(sprites: &crate::imported::Sprites, element: crate::elements::Element, radius: f64) -> f64 {
    let ceiling = radius * 2.0 * VERDICT_INK_SHARE;
    let held = |ink: f32| -> f64 {
        let ink = f64::from(ink);
        if ink > ceiling && ink > 0.0 {
            ceiling / ink
        } else {
            1.0
        }
    };

    let widest = crate::elements::Verdict::ALL
        .iter()
        .filter_map(|verdict| sprites.steady_ink(crate::elements::Element::Verdict(*verdict)))
        .filter(|(wide, high)| *wide > 0.0 && *high > 0.0)
        .map(|(wide, high)| f64::from(wide) * held(high))
        .fold(0.0, f64::max);

    let mine = sprites.steady_ink(element).map_or(1.0, |(_, high)| held(high));
    let room = radius * 2.0 * VERDICT_WIDTH_SHARE;
    if widest > room && widest > 0.0 {
        mine * room / widest
    } else {
        mine
    }
}

fn verdict_place(object: &dossier_sim::TimedObject, at_head: bool) -> Point {
    if at_head {
        return object.pos;
    }
    object.ball_at(object.end_ms).unwrap_or(object.pos)
}

fn verdict_alpha(age: f64) -> f32 {
    if age < VERDICT_FADE_IN_MS {
        (age / VERDICT_FADE_IN_MS) as f32
    } else if age < VERDICT_HOLD_MS {
        1.0
    } else {
        (1.0 - (age - VERDICT_HOLD_MS) / VERDICT_FADE_OUT_MS).clamp(0.0, 1.0) as f32
    }
}

fn miss_drift(age: f64) -> f64 {
    let t = (age / VERDICT_MS).clamp(0.0, 1.0);
    MISS_DRIFT_FROM + MISS_DRIFT_BY * t * t
}

fn verdict_settle(age: f64, missed: bool) -> f32 {
    let step = VERDICT_FADE_IN_MS * 0.2;
    if missed {
        let t = (age / 100.0).clamp(0.0, 1.0) as f32;
        return 1.6 + (1.0 - 1.6) * t * t;
    }
    let ease = |from: f32, to: f32, at: f64, over: f64| {
        from + (to - from) * (at / over).clamp(0.0, 1.0) as f32
    };
    if age < step * 4.0 {
        ease(0.6, 1.1, age, step * 4.0)
    } else if age < step * 5.0 {
        1.1
    } else if age < step * 6.0 {
        ease(1.1, 0.9, age - step * 5.0, step)
    } else if age < step * 7.0 {
        ease(0.9, 1.0, age - step * 6.0, step)
    } else {
        1.0
    }
}

impl Scene<'_> {
    pub(super) fn draw_lighting(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        if !self.skin.hit_lighting || !self.skin_speaks_for(crate::elements::Element::Lighting) {
            return;
        }
        let radius = self.state.difficulty().circle_radius();
        for index in self.candidates(time_ms) {
            let annotation = &self.annotations[index];
            let Some(verdict) = annotation.verdict else {
                continue;
            };
            if verdict == Judgement::Miss {
                continue;
            }
            let age = time_ms - annotation.resolved_ms;
            if !(0.0..LIGHTING_MS).contains(&age) {
                continue;
            }
            let alpha = if age < LIGHTING_FADE_IN_MS {
                (age / LIGHTING_FADE_IN_MS) as f32
            } else if age < LIGHTING_HOLD_MS {
                1.0
            } else {
                (1.0 - (age - LIGHTING_HOLD_MS) / LIGHTING_FADE_OUT_MS).clamp(0.0, 1.0) as f32
            };
            if alpha <= 0.0 {
                continue;
            }

            let t = (age / LIGHTING_GROWTH_MS).clamp(0.0, 1.0) as f32;
            let eased = 1.0 - (1.0 - t) * (1.0 - t);
            let scale = LIGHTING_FROM + (LIGHTING_TO - LIGHTING_FROM) * eased;
            let object = &self.state.timeline().objects[index];
            self.draw_sprite_blended(
                pixmap,
                crate::elements::Element::Lighting,
                annotation.colour,
                verdict_place(object, annotation.judged_before_the_end(object)),
                layout.length(radius) * scale,
                alpha,
                layout,
                0.0,
                tiny_skia::BlendMode::Plus,
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slider(slides: u32) -> dossier_sim::GameState {
        let map = dossier_beatmap::Beatmap::parse(&format!(
            "osu file format v14\n\n[Difficulty]\nCircleSize:5\nApproachRate:5\nSliderMultiplier:1.4\n\
             SliderTickRate:1\n\n[TimingPoints]\n0,500,4,2,0,60,1,0\n\n\
             [HitObjects]\n100,192,1000,2,0,L|240:192,{slides},140\n"
        ))
        .expect("a map");
        dossier_sim::GameState::from_beatmap(&map, dossier_replay::Mods::default())
    }

    #[test]
    fn a_sliders_verdict_is_flashed_where_the_ball_finished() {
        let state = slider(1);
        let object = &state.timeline().objects[0];
        let at = verdict_place(object, false);

        assert!(
            (at.x - 240.0).abs() < 1.0,
            "the mark belongs at the tail (240), not at {}",
            at.x
        );
        assert!(
            (at.x - object.pos.x).abs() > 100.0,
            "and the head is not it"
        );
    }

    #[test]
    fn a_slider_that_comes_back_is_marked_at_its_head() {
        let state = slider(2);
        let object = &state.timeline().objects[0];
        assert!(
            (verdict_place(object, false).x - 100.0).abs() < 1.0,
            "two slides end where they started"
        );
    }

    #[test]
    fn a_circle_is_marked_where_it_is() {
        let map = dossier_beatmap::Beatmap::parse(
            "osu file format v14\n\n[Difficulty]\nCircleSize:4\nApproachRate:5\n\n\
             [TimingPoints]\n0,500,4,2,0,60,1,0\n\n[HitObjects]\n256,192,1000,5,0\n",
        )
        .expect("a map");
        let state = dossier_sim::GameState::from_beatmap(&map, dossier_replay::Mods::default());
        let object = &state.timeline().objects[0];
        assert_eq!(verdict_place(object, false).x, object.pos.x);
        assert_eq!(verdict_place(object, false).y, object.pos.y);
    }

    #[test]
    fn a_verdict_given_at_the_head_is_flashed_at_the_head() {
        let state = slider(1);
        let object = &state.timeline().objects[0];
        let at = verdict_place(object, true);
        assert!((at.x - object.pos.x).abs() < 1.0, "lazer judges the head when it is hit, so the mark sits on it, not at {}", at.x);
    }

    #[test]
    fn a_verdict_holds_half_a_second_before_it_starts_leaving() {
        assert_eq!(verdict_alpha(0.0), 0.0, "it fades in from nothing");
        assert!(
            (verdict_alpha(60.0) - 0.5).abs() < 0.01,
            "halfway in at 60ms"
        );
        assert_eq!(verdict_alpha(120.0), 1.0);
        assert_eq!(verdict_alpha(499.0), 1.0, "full for the whole hold");
        assert!(
            (verdict_alpha(800.0) - 0.5).abs() < 0.01,
            "halfway out at 800ms"
        );
        assert_eq!(verdict_alpha(1100.0), 0.0, "and gone at eleven hundred");
    }

    #[test]
    fn it_outlasts_the_quarter_second_it_used_to_get() {
        assert_eq!(verdict_alpha(240.0), 1.0);
        assert!(
            verdict_alpha(900.0) > 0.0,
            "still readable most of a second on"
        );
    }

    #[test]
    fn a_miss_hangs_where_it_landed_before_it_drops() {
        assert!(
            (miss_drift(0.0) - MISS_DRIFT_FROM).abs() < 1e-6,
            "five pixels above"
        );

        assert!(miss_drift(VERDICT_MS * 0.2) < 0.0, "still above the note");
        assert!(
            miss_drift(VERDICT_MS * 0.5) < MISS_DRIFT_BY * 0.25,
            "a quarter at half"
        );
        assert!(
            (miss_drift(VERDICT_MS) - (MISS_DRIFT_FROM + MISS_DRIFT_BY)).abs() < 1e-6,
            "and seventy-five below by the time it is gone"
        );

        assert!(miss_drift(VERDICT_MS) - miss_drift(VERDICT_MS * 0.9) > 5.0);
    }

    #[test]
    fn a_miss_lands_large_and_snaps_down_while_a_score_springs_up() {
        assert!((verdict_settle(0.0, true) - 1.6).abs() < 0.001);
        assert!((verdict_settle(100.0, true) - 1.0).abs() < 0.001);
        assert!(verdict_settle(50.0, true) > 1.0, "on its way down, not up");

        assert!((verdict_settle(0.0, false) - 0.6).abs() < 0.001);
        assert!(
            (verdict_settle(96.0, false) - 1.1).abs() < 0.001,
            "overshoots"
        );
        assert!(
            (verdict_settle(500.0, false) - 1.0).abs() < 0.001,
            "and settles"
        );
    }

    #[test]
    fn both_are_at_rest_long_before_the_mark_goes() {
        for missed in [true, false] {
            assert!(
                (verdict_settle(200.0, missed) - 1.0).abs() < 0.001,
                "{missed}"
            );
            assert!(
                (verdict_settle(1000.0, missed) - 1.0).abs() < 0.001,
                "{missed}"
            );
        }
    }
}
