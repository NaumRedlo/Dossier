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

    fn sparked(&self, verdict: Judgement) -> Option<crate::elements::Element> {
        if verdict == Judgement::Miss || (verdict == Judgement::Great && !self.skin.show_300) {
            return None;
        }
        let kind = verdict_kind(verdict);
        let particle = crate::elements::Element::Particle(kind);
        let sprites = self.skin.sprites.as_ref()?;
        (!sprites.draw_ourselves(crate::elements::Element::Verdict(kind)) && sprites.get(particle).is_some()).then_some(particle)
    }

    pub(super) fn draw_particles(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let Some(sprites) = self.skin.sprites.as_ref() else {
            return;
        };
        let lazer = self.state.from_lazer();
        let life = if lazer { PARTICLE_LIFE_MS_IN_LAZER } else { PARTICLE_LIFE_MS };
        let unit = if lazer { self.state.difficulty().circle_radius() / NOTE_SPRITE_RADIUS } else { FIELD_SPRITE };
        let reach = layout.length(PARTICLE_REACH * unit);
        let mut stamps: Vec<(crate::elements::Element, Pixmap)> = Vec::new();

        for index in self.candidates(time_ms) {
            let annotation = &self.annotations[index];
            let Some(particle) = annotation.verdict.and_then(|verdict| self.sparked(verdict)) else {
                continue;
            };
            let age = time_ms - annotation.resolved_ms;
            if !(0.0..life).contains(&age) {
                continue;
            }
            if !stamps.iter().any(|(held, _)| *held == particle) {
                let Some((art, per_osu_pixel)) = sprites.coloured(particle, 0) else {
                    continue;
                };
                let scale = layout.length(unit) / per_osu_pixel;
                let Some(stamp) = scaled(art, scale) else {
                    continue;
                };
                stamps.push((particle, stamp));
            }
            let Some((_, stamp)) = stamps.iter().find(|(held, _)| *held == particle) else {
                continue;
            };
            let object = &self.state.timeline().objects[index];
            let (x, y) = layout.map(verdict_place(object, annotation.judged_before_the_end(object)));
            let mut dice = Dice::of(index);
            for _ in 0..PARTICLES {
                let lasts = life * (PARTICLE_SHORTEST_SHARE + (1.0 - PARTICLE_SHORTEST_SHARE) * dice.roll());
                let towards = dice.roll() * std::f64::consts::TAU;
                let far = dice.roll() as f32 * reach;
                let gone = (age / lasts) as f32;
                if gone >= 1.0 {
                    continue;
                }
                let (sin, cos) = towards.sin_cos();
                add_onto(pixmap, stamp, x + cos as f32 * far * gone, y + sin as f32 * far * gone, 1.0 - gone);
            }
        }
    }

    pub(super) fn draw_verdicts(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, tier: Option<Tier>) {
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

            let element = crate::elements::Element::Verdict(verdict_kind(verdict));
            let sparked = self.sparked(verdict).is_some();
            let alpha = if sparked && !self.state.from_lazer() { sparked_alpha(age) } else { verdict_alpha(age) };

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
            let missed = verdict == Judgement::Miss;
            let lazer = self.state.from_lazer();
            let fatal = self.state.mods().contains(dossier_replay::bits::SUDDEN_DEATH) || self.state.mods().contains(dossier_replay::bits::PERFECT);
            let unit = if lazer { radius / NOTE_SPRITE_RADIUS } else { FIELD_SPRITE };
            let drift = if missed && self.skin_version() > 1.0 {
                miss_drift(age, lazer.then_some(unit))
            } else {
                0.0
            };
            let animated = self.skin.sprites.as_ref().is_some_and(|sprites| sprites.animated(element));
            let settle = if animated {
                1.0
            } else if missed {
                miss_settle(age, lazer, fatal)
            } else if sparked {
                sparked_settle(age)
            } else {
                verdict_settle(age)
            };
            if self.skin_speaks_for(element) {
                let Some(sprite) = self.skin.sprites.as_ref().and_then(|sprites| sprites.get(element)) else {
                    continue;
                };
                let own = layout.length(f64::from(sprite.width()) * unit);
                let mut place = verdict_place(object, at_head);
                let mut degrees = 0.0;
                if missed && !animated {
                    place.y += drift;
                    degrees = miss_turn(index, age);
                }
                if tier != Some(if sparked { Tier::Above } else { Tier::Beneath }) {
                    self.draw_sprite_wide_at(pixmap, element, place, own * settle, alpha, layout, degrees, age);
                }
                if sparked && !object.is_spinner() && tier != Some(Tier::Beneath) {
                    let glint = if animated { 1.0 } else { glint_settle(age) };
                    self.draw_sprite_wide_lit_at(pixmap, element, place, own * glint, glint_alpha(age), layout, age);
                }
                continue;
            }
            if tier == Some(Tier::Beneath) {
                continue;
            }
            let mut at = layout.map(verdict_place(object, at_head));
            at.1 += layout.length(drift);
            let size = layout.length(radius * scale) * settle;
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

fn verdict_kind(verdict: Judgement) -> crate::elements::Verdict {
    match verdict {
        Judgement::Great => crate::elements::Verdict::Three,
        Judgement::Ok => crate::elements::Verdict::Hundred,
        Judgement::Meh => crate::elements::Verdict::Fifty,
        Judgement::Miss => crate::elements::Verdict::Miss,
    }
}

struct Dice(u64);

impl Dice {
    fn of(index: usize) -> Self {
        Self((index as u64).wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }

    fn roll(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn scaled(art: &Pixmap, scale: f32) -> Option<Pixmap> {
    let wide = (art.width() as f32 * scale).ceil().max(1.0) as u32;
    let high = (art.height() as f32 * scale).ceil().max(1.0) as u32;
    let mut out = Pixmap::new(wide, high)?;
    let paint = tiny_skia::PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, ..Default::default() };
    out.draw_pixmap(0, 0, art.as_ref(), &paint, Transform::from_scale(wide as f32 / art.width() as f32, high as f32 / art.height() as f32), None);
    Some(out)
}

fn add_onto(onto: &mut Pixmap, stamp: &Pixmap, x: f32, y: f32, alpha: f32) {
    let share = (alpha.clamp(0.0, 1.0) * 256.0) as u32;
    if share == 0 {
        return;
    }
    let (wide, high) = (stamp.width() as i64, stamp.height() as i64);
    let (room_wide, room_high) = (onto.width() as i64, onto.height() as i64);
    let left = (x - wide as f32 / 2.0).round() as i64;
    let top = (y - high as f32 / 2.0).round() as i64;
    let from = stamp.data();
    let into = onto.data_mut();
    for row in (-top).max(0)..high.min(room_high - top) {
        for column in (-left).max(0)..wide.min(room_wide - left) {
            let source = ((row * wide + column) * 4) as usize;
            let target = (((top + row) * room_wide + left + column) * 4) as usize;
            for channel in 0..4 {
                let lit = u32::from(into[target + channel]) + ((u32::from(from[source + channel]) * share) >> 8);
                into[target + channel] = lit.min(255) as u8;
            }
        }
    }
}

fn sparked_alpha(age: f64) -> f32 {
    if age < SPARKED_FADE_IN_MS {
        (age / SPARKED_FADE_IN_MS) as f32
    } else {
        verdict_alpha(age.max(VERDICT_FADE_IN_MS))
    }
}

fn sparked_settle(age: f64) -> f32 {
    SPARKED_FROM + (SPARKED_TO - SPARKED_FROM) * (age / VERDICT_MS).clamp(0.0, 1.0) as f32
}

fn glint_settle(age: f64) -> f32 {
    let step = VERDICT_FADE_IN_MS * 0.2;
    if age < step * 4.0 {
        0.6 + 0.5 * (age / (step * 4.0)).clamp(0.0, 1.0) as f32
    } else if (step * 5.0..step * 6.0).contains(&age) {
        1.1 - 0.2 * ((age - step * 5.0) / step) as f32
    } else {
        sparked_settle(age)
    }
}

fn glint_alpha(age: f64) -> f32 {
    if age < GLINT_FULL_MS {
        let t = ((age - GLINT_FROM_MS) / (GLINT_FULL_MS - GLINT_FROM_MS)).clamp(0.0, 1.0) as f32;
        GLINT_ALPHA * (1.0 - (1.0 - t) * (1.0 - t))
    } else {
        GLINT_ALPHA * (1.0 - (age - GLINT_FULL_MS) / (GLINT_GONE_MS - GLINT_FULL_MS)).clamp(0.0, 1.0) as f32
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

fn miss_drift(age: f64, in_lazer: Option<f64>) -> f64 {
    let t = (age / VERDICT_MS).clamp(0.0, 1.0);
    match in_lazer {
        Some(unit) => (MISS_DRIFT_FROM + MISS_DRIFT_BY_IN_LAZER * t * t) * unit,
        None => MISS_DRIFT_FROM + MISS_DRIFT_BY * t * t,
    }
}

fn miss_settle(age: f64, lazer: bool, fatal: bool) -> f32 {
    if lazer {
        let t = (age / MISS_SETTLE_MS_IN_LAZER).clamp(0.0, 1.0) as f32;
        return MISS_LANDS_AT_IN_LAZER + (1.0 - MISS_LANDS_AT_IN_LAZER) * t * t;
    }
    if fatal {
        let t = (age / MISS_BURST_MS).clamp(0.0, 1.0) as f32;
        return MISS_LANDS_AT + (MISS_BURSTS_TO - MISS_LANDS_AT) * t;
    }
    let t = (age / VERDICT_FADE_IN_MS).clamp(0.0, 1.0) as f32;
    MISS_LANDS_AT + (1.0 - MISS_LANDS_AT) * t
}

fn miss_turn(index: usize, age: f64) -> f32 {
    let dice = (index as u32).wrapping_add(1).wrapping_mul(2_654_435_761) >> 8;
    let lean = MISS_TURN_DEGREES * (dice as f32 / (1u32 << 23) as f32 - 1.0);
    if age < VERDICT_FADE_IN_MS {
        return lean * (age / VERDICT_FADE_IN_MS).max(0.0) as f32;
    }
    let t = ((age - VERDICT_FADE_IN_MS) / (VERDICT_MS - VERDICT_FADE_IN_MS)).clamp(0.0, 1.0) as f32;
    lean * (1.0 + t * t)
}

fn verdict_settle(age: f64) -> f32 {
    let step = VERDICT_FADE_IN_MS * 0.2;
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
        ease(0.95, 1.0, age - step * 6.0, step)
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
        assert!((miss_drift(0.0, None) - MISS_DRIFT_FROM).abs() < 1e-6, "five pixels above");
        assert!(miss_drift(VERDICT_MS * 0.2, None) < 0.0, "still above the note");
        assert!(miss_drift(VERDICT_MS * 0.5, None) < MISS_DRIFT_BY * 0.25, "a quarter at half");
        assert!(
            (miss_drift(VERDICT_MS, None) - 40.0).abs() < 1e-6,
            "the client leaves it forty below by the time it is gone"
        );
        assert!(miss_drift(VERDICT_MS, None) - miss_drift(VERDICT_MS * 0.9, None) > 5.0);
    }

    #[test]
    fn lazer_drops_a_miss_by_the_notes_own_measure() {
        let small = miss_drift(VERDICT_MS, Some(0.25));
        let large = miss_drift(VERDICT_MS, Some(0.5));
        assert!((small - 75.0 * 0.25).abs() < 1e-6, "{small}");
        assert!((large - small * 2.0).abs() < 1e-6, "a note twice the size drops it twice as far");
    }

    #[test]
    fn a_miss_lands_at_twice_its_size_and_is_whole_when_it_has_faded_in() {
        assert!((miss_settle(0.0, false, false) - 2.0).abs() < 0.001);
        assert!((miss_settle(60.0, false, false) - 1.5).abs() < 0.001, "in a straight line");
        assert!((miss_settle(120.0, false, false) - 1.0).abs() < 0.001);
        assert!((miss_settle(1000.0, false, false) - 1.0).abs() < 0.001);
    }

    #[test]
    fn the_miss_that_ends_a_sudden_death_play_bursts() {
        assert!((miss_settle(0.0, false, true) - 2.0).abs() < 0.001);
        assert!((miss_settle(300.0, false, true) - 4.0).abs() < 0.001);
        assert!((miss_settle(900.0, false, true) - 6.0).abs() < 0.001, "and stays there");
    }

    #[test]
    fn lazer_lands_a_miss_smaller_and_sooner() {
        assert!((miss_settle(0.0, true, true) - 1.6).abs() < 0.001, "and knows nothing of sudden death");
        assert!(miss_settle(50.0, true, false) > 1.4, "it eases in, so it is slow to start");
        assert!((miss_settle(100.0, true, false) - 1.0).abs() < 0.001);
    }

    #[test]
    fn a_miss_leans_as_it_arrives_and_twice_as_far_by_the_time_it_is_gone() {
        let mut either = (false, false);
        for index in 0..40 {
            let lean = miss_turn(index, VERDICT_FADE_IN_MS);
            assert!(lean.abs() <= MISS_TURN_DEGREES, "{lean}");
            assert_eq!(miss_turn(index, 0.0), 0.0, "upright as it lands");
            assert!((miss_turn(index, 60.0) - lean / 2.0).abs() < 0.001);
            assert!((miss_turn(index, VERDICT_MS) - lean * 2.0).abs() < 0.001);
            assert_eq!(miss_turn(index, 700.0), miss_turn(index, 700.0), "the same lean at every frame");
            either = (either.0 || lean > 1.0, either.1 || lean < -1.0);
        }
        assert!(either.0 && either.1, "misses lean both ways");
    }

    #[test]
    fn a_score_springs_up_past_its_size_and_settles() {
        assert!((verdict_settle(0.0) - 0.6).abs() < 0.001);
        assert!((verdict_settle(96.0) - 1.1).abs() < 0.001, "overshoots");
        assert!((verdict_settle(110.0) - 1.1).abs() < 0.001, "and holds until it has faded in");
        assert!(verdict_settle(140.0) < 0.95, "then dips");
        assert!((verdict_settle(200.0) - 1.0).abs() < 0.001, "and is at rest long before it goes");
        assert!((verdict_settle(1000.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn a_mark_that_comes_with_particles_is_whole_sooner_and_grows_all_its_life() {
        assert!((sparked_alpha(40.0) - 0.5).abs() < 0.001, "it fades in over eighty milliseconds, not a hundred and twenty");
        assert_eq!(sparked_alpha(80.0), 1.0);
        assert_eq!(sparked_alpha(100.0), 1.0, "and does not dip while the plain mark would still be arriving");
        assert_eq!(sparked_alpha(800.0), verdict_alpha(800.0), "it leaves as every mark leaves");
        assert!((sparked_settle(0.0) - 0.9).abs() < 0.001);
        assert!((sparked_settle(VERDICT_MS) - 1.05).abs() < 0.001);
        assert!(sparked_settle(400.0) > sparked_settle(200.0), "no spring: one slow swell");
    }

    #[test]
    fn its_bright_copy_springs_as_a_plain_mark_does_and_is_gone_in_a_third_of_a_second() {
        assert_eq!(glint_alpha(GLINT_FROM_MS), 0.0);
        assert!((glint_alpha(GLINT_FULL_MS) - GLINT_ALPHA).abs() < 0.001, "half as bright as the mark at its fullest");
        assert!(glint_alpha(0.0) > GLINT_ALPHA * 0.25, "already lit on the frame of the hit");
        assert_eq!(glint_alpha(GLINT_GONE_MS), 0.0);
        assert!((glint_settle(0.0) - 0.6).abs() < 0.001);
        assert!(glint_settle(90.0) > 1.0, "it overshoots");
        assert!((glint_settle(100.0) - sparked_settle(100.0)).abs() < 0.001, "between the client's two steps nothing holds it, and the slow swell shows through");
        assert!((glint_settle(132.0) - 1.0).abs() < 0.001, "half way down the second step");
        assert!((glint_settle(200.0) - sparked_settle(200.0)).abs() < 0.001);
    }

    #[test]
    fn a_hit_throws_the_same_particles_every_time_it_is_drawn() {
        let rolls = |index| {
            let mut dice = Dice::of(index);
            (0..6).map(|_| dice.roll()).collect::<Vec<_>>()
        };
        assert_eq!(rolls(7), rolls(7), "a frame drawn twice is the same frame");
        assert_ne!(rolls(7), rolls(8), "and two hits do not burst alike");
        assert!(rolls(0).iter().chain(rolls(123_456).iter()).all(|roll| (0.0..1.0).contains(roll)));
    }

    #[test]
    fn a_particle_adds_its_light_and_is_cut_at_the_edge_of_the_frame() {
        let mut onto = Pixmap::new(8, 8).unwrap();
        onto.fill(Color::from_rgba8(100, 100, 100, 255));
        let mut stamp = Pixmap::new(4, 4).unwrap();
        stamp.fill(Color::from_rgba8(200, 200, 200, 255));

        add_onto(&mut onto, &stamp, 4.0, 4.0, 0.5);
        let lit = onto.pixel(4, 4).unwrap();
        assert!(lit.red() > 190 && lit.red() < 210, "half of 200 on top of 100: {}", lit.red());
        assert_eq!(onto.pixel(0, 0).unwrap().red(), 100, "nothing outside the stamp is touched");

        add_onto(&mut onto, &stamp, 4.0, 4.0, 1.0);
        assert_eq!(onto.pixel(4, 4).unwrap().red(), 255, "light adds up and stops at white");

        add_onto(&mut onto, &stamp, -10.0, 100.0, 1.0);
        add_onto(&mut onto, &stamp, 0.0, 0.0, 1.0);
        assert_eq!(onto.pixel(0, 0).unwrap().red(), 255, "a stamp half off the frame lights the half that is on it");
        assert_eq!(onto.pixel(7, 7).unwrap().red(), 100);
    }
}
