use super::paint::{draw_bar, draw_pill, pie};
use super::*;

const SCORE_SIZE: f64 = 0.050;

const SCORE_OF_FACE: f32 = 0.96;
const COMBO_OF_FACE: f32 = 1.28;

const HUD_SPACE: f64 = 768.0;
const ACCURACY_OF_SCORE: f32 = 0.6;
const COMBO_OF_SCORE: f32 = 1.28 / 0.96;

const PROGRESS_RADIUS: f64 = 16.0 / 768.0;

const PROGRESS_GAP: f32 = 0.5;
const EDGE_MARGIN: f64 = 12.8 / 768.0;

const OUR_BAR_WIDTH: f32 = 0.325;

const FILL_OFFSET: (f32, f32) = (3.0 * 1.6, 10.0 * 1.6);

use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::layout::Layout;
use crate::skin::with_alpha;
use crate::text::{Align, Label};

impl Scene<'_> {
    #[allow(clippy::type_complexity)]

    fn hud_glyphs(
        &self,
        text: &str,
        height: f32,
        combo_face: bool,
    ) -> Option<(Vec<Option<(&tiny_skia::Pixmap, f32)>>, f32, f32)> {
        let sprites = self.skin.sprites.as_ref()?;
        let mut art = Vec::with_capacity(text.len());
        let (mut asked, mut answered) = (0usize, 0usize);
        for glyph in text.chars() {
            if glyph == ' ' {
                art.push(None);
                continue;
            }
            let element = if combo_face {
                crate::elements::Element::Combo(glyph)
            } else {
                crate::elements::Element::Score(glyph)
            };
            if sprites.silenced(element) {
                continue;
            }
            asked += 1;
            match sprites.coloured(element, 0) {
                Some(picture) => {
                    answered += 1;
                    art.push(Some(picture));
                }

                None => continue,
            }
        }

        if asked > 0 && answered == 0 {
            return None;
        }

        let tallest = art
            .iter()
            .flatten()
            .map(|(pixmap, per)| pixmap.height() as f32 / per)
            .fold(0.0f32, f32::max)
            .max(1.0);
        let scale = height / tallest;
        let overlap = if combo_face {
            sprites.ini().combo_overlap
        } else {
            sprites.ini().score_overlap
        };
        let width: f32 = art
            .iter()
            .map(|glyph| match glyph {
                Some((pixmap, per)) => pixmap.width() as f32 / per - overlap,

                None => height / scale / 3.0,
            })
            .sum::<f32>()
            + overlap;
        Some((art, scale, width * scale))
    }

    fn hud_face_height(&self, combo_face: bool) -> Option<f32> {
        let sprites = self.skin.sprites.as_ref()?;
        let tallest = ('0'..='9')
            .filter_map(|digit| {
                let element = if combo_face {
                    crate::elements::Element::Combo(digit)
                } else {
                    crate::elements::Element::Score(digit)
                };
                sprites
                    .coloured(element, 0)
                    .map(|(pixmap, per)| pixmap.height() as f32 / per)
            })
            .fold(0.0f32, f32::max);
        (tallest > 0.0).then_some(tallest)
    }

    pub(super) fn hud_text_width(&self, text: &str, height: f32) -> f32 {
        if let Some((_, _, width)) = self.hud_glyphs(text, height, false) {
            return width;
        }
        self.skin
            .font
            .as_ref()
            .map_or(0.0, |font| font.width(text, height))
    }

    pub(super) fn draw_hud_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        right_x: f32,
        baseline_y: f32,
        height: f32,
        align: Align,
        alpha: f32,
    ) -> bool {
        self.draw_hud_glyphs(
            pixmap, text, right_x, baseline_y, height, align, alpha, None, false, false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_hud_text_in(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        right_x: f32,
        baseline_y: f32,
        height: f32,
        align: Align,
        alpha: f32,
        colour: tiny_skia::Color,
    ) -> bool {
        self.draw_hud_glyphs(
            pixmap,
            text,
            right_x,
            baseline_y,
            height,
            align,
            alpha,
            Some(colour),
            false,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_hud_glyphs(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        right_x: f32,
        baseline_y: f32,
        height: f32,
        align: Align,
        alpha: f32,
        tint: Option<tiny_skia::Color>,
        combo_face: bool,
        additive: bool,
    ) -> bool {
        let Some((art, scale, width)) = self.hud_glyphs(text, height, combo_face) else {
            return false;
        };
        if art.is_empty() {
            return true;
        }
        let overlap = self.skin.sprites.as_ref().map_or(0.0, |s| {
            if combo_face {
                s.ini().combo_overlap
            } else {
                s.ini().score_overlap
            }
        });
        let mut pen = match align {
            Align::Right => right_x - width,
            Align::Centre => right_x - width / 2.0,
            Align::Left => right_x,
        };
        for glyph in art {
            let Some((art_pixmap, per)) = glyph else {
                pen += height / 3.0;
                continue;
            };
            let each = scale / per;
            let drawn_height = art_pixmap.height() as f32 * each;
            let transform =
                Transform::from_translate(pen, baseline_y - drawn_height).pre_scale(each, each);

            let painted = tint.map(|colour| crate::imported::tinted(art_pixmap, colour));
            pixmap.draw_pixmap(
                0,
                0,
                painted.as_ref().unwrap_or(art_pixmap).as_ref(),
                &PixmapPaint {
                    opacity: alpha.clamp(0.0, 1.0),
                    quality: tiny_skia::FilterQuality::Bilinear,
                    blend_mode: if additive {
                        tiny_skia::BlendMode::Plus
                    } else {
                        tiny_skia::BlendMode::SourceOver
                    },
                },
                transform,
                None,
            );
            pen += (art_pixmap.width() as f32 / per - overlap) * scale;
        }
        true
    }

    fn draw_combo(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        (x, baseline): (f32, f32),
        size: f32,
        alpha: f32,
        additive: bool,
    ) {
        if alpha <= 0.01 || size <= 0.0 {
            return;
        }
        if self.draw_hud_glyphs(
            pixmap,
            text,
            x,
            baseline,
            size,
            Align::Left,
            alpha,
            None,
            true,
            additive,
        ) {
            return;
        }
        if additive {
            return;
        }
        let Some(font) = &self.skin.font else {
            return;
        };
        font.draw(
            pixmap,
            Label {
                text,
                x,
                y: baseline,
                size,
                colour: with_alpha(self.skin.hud, alpha),
                align: Align::Left,
            },
        );
    }

    pub(super) fn draw_hud(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let (Some(font), Some(judge)) = (&self.skin.font, self.state.judge()) else {
            return;
        };

        let presence = self.hud_presence(time_ms);
        let score = judge.state_at(time_ms);
        let height = f64::from(layout.height);
        let margin = (height * EDGE_MARGIN) as f32;

        let to_screen = (height / HUD_SPACE) as f32;
        let score_size = self
            .hud_face_height(false)
            .map_or((height * SCORE_SIZE) as f32, |own| {
                own * SCORE_OF_FACE * to_screen
            });
        let accuracy_size = score_size * ACCURACY_OF_SCORE;

        let leads = if self.state.score_at(time_ms).is_some() {
            score_size
        } else {
            accuracy_size
        };
        let mut top = self.top_band(layout) + font.digit_height(leads) / 2.0 - leads;
        if let Some(value) = self.state.score_at(time_ms) {
            let text = value.to_string();
            let right = layout.width as f32 - margin;
            if !self.draw_hud_text(
                pixmap,
                &text,
                right,
                top + score_size,
                score_size,
                Align::Right,
                1.0,
            ) {
                font.draw(
                    pixmap,
                    Label {
                        text: &text,
                        x: right,
                        y: top + score_size,
                        size: score_size,
                        colour: self.skin.hud,
                        align: Align::Right,
                    },
                );
            }
            top += score_size * 1.15;
        }
        let accuracy = format!("{:.2}%", score.accuracy());
        let right = layout.width as f32 - margin;
        if !self.draw_hud_text(
            pixmap,
            &accuracy,
            right,
            top + accuracy_size,
            accuracy_size,
            Align::Right,
            1.0,
        ) {
            font.draw(
                pixmap,
                Label {
                    text: &accuracy,
                    x: right,
                    y: top + accuracy_size,
                    size: accuracy_size,
                    colour: self.skin.hud,
                    align: Align::Right,
                },
            );
        }

        let radius = (height * PROGRESS_RADIUS) as f32;
        let widest = self.hud_text_width("100.00%", accuracy_size);
        self.draw_progress(
            pixmap,
            time_ms,
            right - widest - accuracy_size * PROGRESS_GAP - radius,
            top + accuracy_size - font.digit_height(accuracy_size) / 2.0,
            radius,
            1.0,
        );

        let combo_face = self
            .hud_face_height(true)
            .map_or(score_size * COMBO_OF_SCORE, |own| {
                own * COMBO_OF_FACE * to_screen
            });
        let bottom = layout.height as f32 - margin;
        let (shown, _, _) = self.combo_shown(time_ms);

        if let Some((popped, swell, ghost)) = self.combo_ghost(time_ms) {
            self.draw_combo(
                pixmap,
                &format!("{popped}x"),
                (margin, bottom),
                combo_face * swell,
                ghost,
                true,
            );
        }
        self.draw_combo(
            pixmap,
            &format!("{shown}x"),
            (margin, bottom),
            combo_face * self.combo_pulse(time_ms),
            1.0,
            false,
        );

        let tally_size = (height * 0.030) as f32;
        let counts = score.counts;
        let tally = [
            (u32::from(counts.count_300), self.skin.verdict_300),
            (u32::from(counts.count_100), self.skin.verdict_100),
            (u32::from(counts.count_50), self.skin.verdict_50),
            (u32::from(counts.count_miss), self.skin.verdict_miss),
        ];
        let mut y = top + accuracy_size + tally_size * 1.6;
        let right_edge = layout.width as f32 - margin;
        for (value, colour) in tally {
            let text = format!("{value}");
            if !self.draw_hud_text_in(
                pixmap,
                &text,
                right_edge,
                y,
                tally_size,
                Align::Right,
                presence,
                colour,
            ) {
                font.draw(
                    pixmap,
                    Label {
                        text: &text,
                        x: right_edge,
                        y,
                        size: tally_size,
                        colour: with_alpha(colour, presence),
                        align: Align::Right,
                    },
                );
            }
            y += tally_size * 1.25;
        }

        self.draw_health(pixmap, time_ms, layout, presence);

        let spinning = self.spinner_grip(time_ms);
        if spinning < 1.0 {
            self.draw_error_bar(pixmap, time_ms, layout, presence * (1.0 - spinning));
        }
        if spinning > 0.0 {
            self.draw_spin_readout(pixmap, time_ms, layout, presence * spinning);
        }
    }

    pub(super) fn draw_signature(&self, pixmap: &mut Pixmap, layout: &Layout) {
        let (Some(font), Some(signature)) = (&self.skin.font, &self.signature) else {
            return;
        };
        let height = f64::from(layout.height);
        let margin = (height * EDGE_MARGIN) as f32;
        let client_size = (height * 0.028) as f32;
        let version_size = (height * 0.015) as f32;

        let bottom = layout.height as f32 - margin;
        font.draw(
            pixmap,
            Label {
                text: &signature.version,
                x: layout.width as f32 - margin,
                y: bottom,
                size: version_size,
                colour: with_alpha(self.skin.hud, 0.20),
                align: Align::Right,
            },
        );
        font.draw(
            pixmap,
            Label {
                text: &signature.client,
                x: layout.width as f32 - margin,
                y: bottom - version_size * 1.15,
                size: client_size,
                colour: with_alpha(self.skin.hud, 0.30),
                align: Align::Right,
            },
        );
        let badges: Vec<String> = signature
            .badges
            .iter()
            .filter(|one| crate::mods::known(one))
            .cloned()
            .collect();
        let high = (client_size * 1.55).round().max(8.0) as u32;
        if let Some(strip) = crate::mods::row(&badges, high) {
            let top = bottom - version_size * 1.15 - client_size * 1.35 - strip.height() as f32;
            pixmap.draw_pixmap(
                0,
                0,
                strip.as_ref(),
                &tiny_skia::PixmapPaint {
                    opacity: 0.9,
                    quality: tiny_skia::FilterQuality::Bilinear,
                    ..Default::default()
                },
                Transform::from_translate(
                    layout.width as f32 - margin - strip.width() as f32,
                    top.max(0.0),
                ),
                None,
            );
        } else if !signature.mods.is_empty() {
            font.draw(
                pixmap,
                Label {
                    text: &signature.mods,
                    x: layout.width as f32 - margin,
                    y: bottom - version_size * 1.15 - client_size * 1.35,
                    size: client_size * 1.35,
                    colour: with_alpha(self.skin.hud, 0.80),
                    align: Align::Right,
                },
            );
        }
    }

    pub(super) fn hud_presence(&self, time_ms: f64) -> f32 {
        let mut presence = 1.0f32;
        for &(from, to) in &self.state.timeline().breaks {
            if to - from < BREAK_HUD_FADE_MS * 2.0 {
                continue;
            }
            if time_ms < from || time_ms > to {
                continue;
            }
            let into = ((time_ms - from) / BREAK_HUD_FADE_MS).clamp(0.0, 1.0) as f32;
            let out_of = ((to - time_ms) / BREAK_HUD_FADE_MS).clamp(0.0, 1.0) as f32;
            presence = presence.min(1.0 - into.min(out_of));
        }
        presence
    }

    fn top_band(&self, layout: &Layout) -> f32 {
        layout.height as f32 * 0.042
    }

    fn draw_progress(
        &self,
        pixmap: &mut Pixmap,
        time_ms: f64,
        cx: f32,
        cy: f32,
        radius: f32,
        presence: f32,
    ) {
        let (from, to) = self.state.span_ms();
        if to <= from || radius <= 0.5 {
            return;
        }
        let played = (((time_ms - from) / (to - from)).clamp(0.0, 1.0)) as f32;

        crate::elements::ring(
            pixmap,
            cx,
            cy,
            radius,
            (radius * 0.07).max(1.0),
            self.skin.hud,
            0.22 * presence,
        );
        pie(
            pixmap,
            cx,
            cy,
            radius * 0.88,
            played,
            with_alpha(self.skin.hud, 0.45 * presence),
        );
    }

    fn draw_health(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, presence: f32) {
        if self.cannot_die() {
            return;
        }
        let Some(health) = self.state.health_at(time_ms) else {
            return;
        };
        if self.draw_skin_health(pixmap, health, presence, layout) {
            return;
        }

        let height = f64::from(layout.height);
        let margin = (height * EDGE_MARGIN) as f32;
        let width = layout.width as f32 * OUR_BAR_WIDTH;
        let thickness = (height * 0.018).max(5.0) as f32;
        let y = self.top_band(layout) - thickness / 2.0;
        draw_pill(
            pixmap,
            margin,
            y,
            width,
            thickness,
            with_alpha(self.skin.hud, 0.13 * presence),
        );

        let (colour, alpha) = if health < 0.33 {
            (self.skin.verdict_miss, 0.95)
        } else {
            (self.skin.hud, 0.62)
        };
        draw_pill(
            pixmap,
            margin,
            y,
            width * health,
            thickness,
            with_alpha(colour, alpha * presence),
        );
    }

    fn draw_skin_health(
        &self,
        pixmap: &mut Pixmap,
        health: f32,
        presence: f32,
        layout: &Layout,
    ) -> bool {
        use crate::elements::Element;
        let fill = Element::ScoreBarFill;
        let frame = Element::ScoreBarBackground;

        if ![fill, frame]
            .iter()
            .any(|&piece| self.skin_speaks_for(piece))
        {
            return false;
        }
        let Some(sprites) = &self.skin.sprites else {
            return false;
        };
        let alpha = presence.clamp(0.0, 1.0);
        let health = health.clamp(0.0, 1.0);

        self.blit_bar(
            pixmap,
            frame,
            0.0,
            0.0,
            self.bar_share(frame),
            alpha,
            layout,
        );

        let at = (
            self.skin_pixels(layout, FILL_OFFSET.0),
            self.skin_pixels(layout, FILL_OFFSET.1),
        );
        self.blit_bar(pixmap, fill, at.0, at.1, health, alpha, layout);

        let mark = Element::ScoreBarMark(crate::elements::Health::of(health));
        if let (Some(shape), true) = (sprites.get(fill), self.skin_speaks_for(mark)) {
            let along = self.skin_pixels(layout, shape.width()) * health;
            self.blit_mark(pixmap, mark, at.0 + along, at.1, alpha, layout);
        }
        true
    }

    fn bar_share(&self, element: crate::elements::Element) -> f32 {
        let Some(sprites) = &self.skin.sprites else {
            return 1.0;
        };
        let Some((art, _)) = sprites.coloured(element, 0) else {
            return 1.0;
        };
        let (wide, tall) = (art.width(), art.height());
        let opaque = |x: u32| {
            (0..tall).step_by(3).any(|y| {
                art.pixels()
                    .get((y * wide + x) as usize)
                    .is_some_and(|p| p.alpha() > 20)
            })
        };
        let least = (wide / 20).max(8);
        let (mut run, mut last) = (0u32, 0u32);
        for x in 0..wide {
            if opaque(x) {
                if run >= least && last > 0 {
                    return last as f32 / wide as f32;
                }
                run = 0;
                last = x + 1;
            } else {
                run += 1;
            }
        }
        1.0
    }

    #[allow(clippy::too_many_arguments)]
    fn blit_bar(
        &self,
        pixmap: &mut Pixmap,
        element: crate::elements::Element,
        x: f32,
        y: f32,
        share: f32,
        alpha: f32,
        layout: &Layout,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };
        let Some((art, per)) = sprites.coloured(element, 0) else {
            return;
        };
        let share = share.clamp(0.0, 1.0);
        if share <= 0.0 || alpha <= 0.0 {
            return;
        }

        let scale = layout.height as f32 / 768.0 / per;
        let full = (art.width() as f32 * scale, art.height() as f32 * scale);
        let visible = (full.0 * share).ceil().max(1.0) as u32;
        let Some(mut strip) = Pixmap::new(visible, full.1.ceil().max(1.0) as u32) else {
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
            Transform::from_scale(scale, scale),
            None,
        );
        pixmap.draw_pixmap(
            x as i32,
            y as i32,
            strip.as_ref(),
            &PixmapPaint {
                opacity: alpha,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }

    fn blit_mark(
        &self,
        pixmap: &mut Pixmap,
        element: crate::elements::Element,
        x: f32,
        y: f32,
        alpha: f32,
        layout: &Layout,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };
        let Some((art, per)) = sprites.coloured(element, 0) else {
            return;
        };
        let scale = layout.height as f32 / 768.0 / per;
        let (w, h) = (art.width() as f32 * scale, art.height() as f32 * scale);
        pixmap.draw_pixmap(
            0,
            0,
            art.as_ref(),
            &PixmapPaint {
                opacity: alpha,
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            Transform::from_translate(x - w / 2.0, y - h / 2.0).pre_scale(scale, scale),
            None,
        );
    }

    fn spinner_grip(&self, time_ms: f64) -> f32 {
        let mut grip: f32 = 0.0;
        for object in &self.state.timeline().objects {
            if !object.is_spinner() {
                continue;
            }

            let opening = ((time_ms - (object.start_ms - SPIN_SWAP_MS)) / SPIN_SWAP_MS) as f32;
            let closing = (((object.end_ms + SPIN_SWAP_MS) - time_ms) / SPIN_SWAP_MS) as f32;
            grip = grip.max(opening.clamp(0.0, 1.0).min(closing.clamp(0.0, 1.0)));
        }
        grip
    }

    fn draw_spin_readout(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, presence: f32) {
        if self
            .skin
            .sprites
            .as_ref()
            .is_some_and(|s| s.silenced(crate::elements::Element::SpinnerRpm))
        {
            return;
        }
        let Some(font) = self.skin.font.as_ref() else {
            return;
        };

        let Some(object) = self
            .state
            .timeline()
            .objects
            .iter()
            .filter(|object| object.is_spinner())
            .min_by(|a, b| {
                let near = |o: &TimedObject| {
                    if time_ms < o.start_ms {
                        o.start_ms - time_ms
                    } else if time_ms > o.end_ms {
                        time_ms - o.end_ms
                    } else {
                        0.0
                    }
                };
                near(a).total_cmp(&near(b))
            })
        else {
            return;
        };
        let rpm = dossier_sim::spinner_rpm(
            self.state.cursor_track(),
            object.start_ms,
            time_ms.clamp(object.start_ms, object.end_ms),
            self.spins_by_itself(),
        );
        let height = f64::from(layout.height);
        let size = (height * SPIN_READOUT_SIZE) as f32;
        let baseline = (height * 0.962) as f32;
        let figure = format!("{rpm:.0}");

        if self.skin_speaks_for(crate::elements::Element::SpinnerRpm) {
            let sprite = self
                .skin
                .sprites
                .as_ref()
                .and_then(|s| s.coloured(crate::elements::Element::SpinnerRpm, 0));
            if let Some((art, per)) = sprite {
                let rise = f64::from(1.0 - presence.clamp(0.0, 1.0)) * SPIN_RPM_RISE;
                let scale = (layout.scale() * SPIN_SPRITE) as f32 / per;
                let (left, top) = layout.map(spin_place(SPIN_RPM_X, SPIN_RPM_Y + rise));

                pixmap.draw_pixmap(
                    0,
                    0,
                    art.as_ref(),
                    &PixmapPaint {
                        opacity: presence.clamp(0.0, 1.0),
                        quality: tiny_skia::FilterQuality::Bilinear,
                        ..Default::default()
                    },
                    Transform::from_translate(left, top).pre_scale(scale, scale),
                    None,
                );

                let glyph = self
                    .hud_face_height(false)
                    .map_or(SPIN_SPM_GLYPH, f64::from);
                let tall =
                    (layout.scale() * SPIN_SPRITE * SPIN_SPM_FACE * glyph) as f32;
                let (right, atop) = layout.map(spin_place(SPIN_SPM_X, SPIN_SPM_Y + rise));
                if !self.draw_hud_text(
                    pixmap,
                    &figure,
                    right,
                    atop + tall,
                    tall,
                    Align::Right,
                    presence,
                ) {
                    font.draw(
                        pixmap,
                        Label {
                            text: &figure,
                            x: right,
                            y: atop + tall,
                            size: tall,
                            colour: with_alpha(self.skin.spinner, presence),
                            align: Align::Right,
                        },
                    );
                }
                return;
            }
        }

        font.draw(
            pixmap,
            Label {
                text: &format!("RPM: {rpm:.0}"),
                x: layout.width as f32 * 0.5,
                y: baseline,
                size,
                colour: with_alpha(self.skin.spinner, presence),
                align: Align::Centre,
            },
        );
    }

    fn draw_error_bar(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, presence: f32) {
        let Some(judge) = self.state.judge() else {
            return;
        };
        let difficulty = self.state.difficulty();
        let (w300, w100, w50) = (
            difficulty.hit_window_300(),
            difficulty.hit_window_100(),
            difficulty.hit_window_50(),
        );
        if w50 <= 0.0 {
            return;
        }

        let scale = self.skin.meter_scale;
        let height = f64::from(layout.height);
        let full_width = (layout.width as f64 * 0.22) as f32 * scale;
        let centre_x = layout.width as f32 * 0.5;
        let y = (height * 0.955) as f32;
        let band = (height * 0.006).max(2.0) as f32 * scale;
        let span = w50 * ERROR_BAR_SPAN;
        let half = |window: f64| (window / span) as f32 * full_width * 0.5;

        for (window, colour) in [
            (w50, self.skin.verdict_50),
            (w100, self.skin.verdict_100),
            (w300, self.skin.verdict_300),
        ] {
            let w = half(window);
            draw_bar(
                pixmap,
                centre_x - w,
                y,
                w * 2.0,
                band,
                with_alpha(colour, 0.30 * presence),
            );
        }

        let mut recent: Vec<(f64, f64)> =
            judge.errors_ms().filter(|&(at, _)| at <= time_ms).collect();

        recent.reverse();
        recent.truncate(ERROR_BAR_TICKS);
        let tick_w = (height * 0.0035).max(1.0) as f32 * scale;
        for (i, (_, error)) in recent.iter().enumerate() {
            let age = i as f32 / ERROR_BAR_TICKS as f32;
            let offset = (*error / span).clamp(-1.0, 1.0) as f32 * full_width * 0.5;
            let colour = if error.abs() < w300 {
                self.skin.verdict_300
            } else if error.abs() < w100 {
                self.skin.verdict_100
            } else {
                self.skin.verdict_50
            };
            draw_bar(
                pixmap,
                centre_x + offset - tick_w * 0.5,
                y - band * 1.6,
                tick_w,
                band * 4.2,
                with_alpha(colour, (1.0 - age) * 0.9 * presence),
            );
        }

        let centre_top = y - band * 2.4;
        draw_bar(
            pixmap,
            centre_x - tick_w * 0.5,
            centre_top,
            tick_w,
            band * 5.8,
            with_alpha(self.skin.hud, 0.75 * presence),
        );

        if let Some(rate) = judge
            .unstable_rate(time_ms)
            .filter(|_| self.skin.unstable_rate)
        {
            let size = (height * ERROR_BAR_UR_SIZE) as f32 * scale;
            let baseline = centre_top - size * ERROR_BAR_UR_GAP;

            let text = format!("{rate:.0}");

            if let Some(font) = self.skin.font.as_ref() {
                font.draw(
                    pixmap,
                    Label {
                        text: &text,
                        x: centre_x,
                        y: baseline,
                        size,
                        colour: with_alpha(self.skin.hud, 0.75 * presence),
                        align: Align::Centre,
                    },
                );
            }
        }
    }
}
