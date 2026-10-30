use super::format::{compact, name_size};
use super::paint::rounded_rect;
use super::*;

use tiny_skia::{Color, FillRule, Paint, Pixmap, Transform};

use crate::layout::Layout;
use crate::skin::{darken, lighten, with_alpha};
use crate::text::{Align, Label};

impl Scene<'_> {
    pub(super) fn draw_leaderboard(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let Some(font) = &self.skin.font else {
            return;
        };
        let height = f64::from(layout.height);
        let size = (height * BOARD_TEXT) as f32;
        let step = (height * BOARD_STEP) as f32;
        let width = (height * BOARD_WIDTH) as f32;
        let (field_left, _) = layout.map(dossier_beatmap::Point { x: 0.0, y: 0.0 });
        let left = ((field_left - width) * BOARD_ROOM_SHARE).max((height * BOARD_LEFT) as f32);
        let card_height = step * BOARD_CARD_FILL;

        let pp = self.leaderboard.pp_at(time_ms);
        let graph = !self.leaderboard.strains.is_empty();
        let rows = match self.state.score_track() {
            Some(track) if !self.leaderboard.is_empty() => self.leaderboard.standings(track.at(time_ms), BOARD_ROWS),
            _ => Vec::new(),
        };
        let gauges = if graph { step * GAUGE_GRAPH } else { 0.0 } + if pp.is_some() { step * GAUGE_PP } else { 0.0 };
        let drawn = BOARD_ROWS.min(rows.len()) as f32;
        let mut y = (pixmap.height() as f32 - gauges - drawn * step) / 2.0;

        if let Some(pp) = pp {
            font.draw(
                pixmap,
                Label {
                    text: &format!("{} pp", pp.round() as u32),
                    x: left,
                    y: y + step * GAUGE_PP * 0.62,
                    size: size * 1.7,
                    colour: self.skin.hud,
                    align: Align::Left,
                },
            );
            y += step * GAUGE_PP;
        }
        if graph {
            self.draw_strains(pixmap, left, y, width, step * GAUGE_GRAPH * 0.86, time_ms);
            y += step * GAUGE_GRAPH;
        }
        let top = y + (drawn - 1.0) * step;
        for row in &rows {
            let baseline = top - row.slot * step + size * 1.15;
            self.draw_board_row(pixmap, font, row, left, baseline, width, card_height, size);
        }
    }

    fn draw_strains(&self, pixmap: &mut Pixmap, left: f32, top: f32, width: f32, height: f32, time_ms: f64) {
        let strains = &self.leaderboard.strains;
        let (from, to) = self.leaderboard.span_ms;
        let played = (((time_ms - from) / (to - from).max(1.0)).clamp(0.0, 1.0)) as f32;
        let bottom = top + height;
        let last = (strains.len().max(2) - 1) as f32;
        let height_at = |x: f32| {
            let place = x.clamp(0.0, 1.0) * last;
            let below = place.floor() as usize;
            let above = (below + 1).min(strains.len() - 1);
            let share = place - below as f32;
            let at = |index: usize| strains.get(index).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            at(below) + (at(above) - at(below)) * share
        };
        let ridge = |from_x: f32, to_x: f32, closed: bool| {
            let mut path = tiny_skia::PathBuilder::new();
            let start = bottom - height_at(from_x) * height;
            if closed {
                path.move_to(left + from_x * width, bottom);
                path.line_to(left + from_x * width, start);
            } else {
                path.move_to(left + from_x * width, start);
            }
            for at in 0..strains.len() {
                let x = at as f32 / last;
                if x > from_x && x < to_x {
                    path.line_to(left + x * width, bottom - height_at(x) * height);
                }
            }
            path.line_to(left + to_x * width, bottom - height_at(to_x) * height);
            if closed {
                path.line_to(left + to_x * width, bottom);
                path.close();
            }
            path.finish()
        };
        let wash = |strong: f32, weak: f32| {
            tiny_skia::LinearGradient::new(
                tiny_skia::Point::from_xy(left, top),
                tiny_skia::Point::from_xy(left, bottom),
                vec![
                    tiny_skia::GradientStop::new(0.0, with_alpha(self.skin.hud, strong)),
                    tiny_skia::GradientStop::new(1.0, with_alpha(self.skin.hud, weak)),
                ],
                tiny_skia::SpreadMode::Pad,
                Transform::identity(),
            )
        };
        let thin = height * GAUGE_LINE;
        for (from_x, to_x, strong, weak, line) in [(played, 1.0, 0.2, 0.02, 0.32), (0.0, played, 0.62, 0.1, 1.0)] {
            if to_x - from_x < 0.001 {
                continue;
            }
            if let (Some(body), Some(shader)) = (ridge(from_x, to_x, true), wash(strong, weak)) {
                let paint = Paint { shader, anti_alias: true, ..Default::default() };
                pixmap.fill_path(&body, &paint, FillRule::Winding, Transform::identity(), None);
            }
            if let Some(edge) = ridge(from_x, to_x, false) {
                let mut paint = Paint { anti_alias: true, ..Default::default() };
                paint.set_color(with_alpha(self.skin.hud, line));
                let stroke = Stroke { width: thin, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
                pixmap.stroke_path(&edge, &paint, &stroke, Transform::identity(), None);
            }
        }
        if played > 0.0 && played < 1.0 {
            let x = left + played * width;
            let mark = tiny_skia::Rect::from_xywh(x - thin / 2.0, bottom - height_at(played) * height, thin, height_at(played) * height);
            if let Some(mark) = mark {
                let mut paint = Paint { anti_alias: true, ..Default::default() };
                paint.set_color(with_alpha(self.skin.hud, 0.9));
                pixmap.fill_rect(mark, &paint, Transform::identity(), None);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_board_row(
        &self,
        pixmap: &mut Pixmap,
        font: &crate::text::Font,
        row: &crate::leaderboard::Row,
        left: f32,
        baseline: f32,
        width: f32,
        card_height: f32,
        size: f32,
    ) {
        let presence = 1.0;
        let top = baseline - size * 1.15;
        let Some(card) = rounded_rect(left, top, width, card_height, card_height * BOARD_RADIUS)
        else {
            return;
        };

        let mut paint = Paint {
            anti_alias: true,
            ..Default::default()
        };
        let has_cover = row
            .entry
            .cover
            .as_deref()
            .is_some_and(|p| self.pictures.contains_key(p));
        if let Some(cover) = row
            .entry
            .cover
            .as_deref()
            .and_then(|p| self.pictures.get(p))
        {
            let scale = (width / cover.width() as f32).max(card_height / cover.height() as f32);
            let shader = tiny_skia::Pattern::new(
                cover.as_ref(),
                tiny_skia::SpreadMode::Pad,
                tiny_skia::FilterQuality::Bilinear,
                presence,
                Transform::from_translate(left, top).pre_scale(scale, scale),
            );
            paint.shader = shader;
            pixmap.fill_path(
                &card,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            paint.shader = Shader::SolidColor(Color::BLACK);
        }

        let base = if row.is_player {
            lighten(self.skin.background, BOARD_CARD_LIFT)
        } else {
            self.skin.background
        };

        let (heavy, light) = if has_cover {
            (BOARD_DARK_LEFT_COVER, BOARD_DARK_RIGHT_COVER)
        } else {
            (BOARD_DARK_LEFT, BOARD_DARK_RIGHT)
        };
        if let Some(shade) = tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(left, top),
            tiny_skia::Point::from_xy(left + width, top),
            vec![
                tiny_skia::GradientStop::new(0.0, with_alpha(base, heavy * presence)),
                tiny_skia::GradientStop::new(
                    BOARD_DARK_SPLIT,
                    with_alpha(base, heavy * BOARD_DARK_KNEE * presence),
                ),
                tiny_skia::GradientStop::new(1.0, with_alpha(base, light * presence)),
            ],
            tiny_skia::SpreadMode::Pad,
            Transform::identity(),
        ) {
            let wash = Paint {
                shader: shade,
                anti_alias: true,
                ..Default::default()
            };
            pixmap.fill_path(&card, &wash, FillRule::Winding, Transform::identity(), None);
        }

        let colour = if row.is_player {
            self.skin.hud
        } else {
            darken(self.skin.hud, BOARD_RIVAL_DIM)
        };

        let face = card_height * BOARD_FACE;
        let face_x = left + card_height * 0.16;
        let face_y = top + (card_height - face) / 2.0;
        if let Some(avatar) = row
            .entry
            .avatar
            .as_deref()
            .and_then(|p| self.pictures.get(p))
        {
            if let Some(clip) = rounded_rect(face_x, face_y, face, face, face * 0.28) {
                let scale = face / avatar.width().max(1) as f32;
                let mut art = Paint {
                    anti_alias: true,
                    ..Default::default()
                };
                art.shader = tiny_skia::Pattern::new(
                    avatar.as_ref(),
                    tiny_skia::SpreadMode::Pad,
                    tiny_skia::FilterQuality::Bilinear,
                    presence,
                    Transform::from_translate(face_x, face_y).pre_scale(scale, scale),
                );
                pixmap.fill_path(&clip, &art, FillRule::Winding, Transform::identity(), None);
            }
        } else if let Some(clip) = rounded_rect(face_x, face_y, face, face, face * 0.28) {
            let mut plain = Paint { anti_alias: true, ..Default::default() };
            plain.set_color(with_alpha(self.skin.hud, 0.14));
            pixmap.fill_path(&clip, &plain, FillRule::Winding, Transform::identity(), None);
            let initial: String = row.entry.name.chars().take(1).flat_map(char::to_uppercase).collect();
            let letter = face * 0.5;
            font.draw(
                pixmap,
                Label {
                    text: &initial,
                    x: face_x + (face - font.width(&initial, letter)) / 2.0,
                    y: face_y + face * 0.68,
                    size: letter,
                    colour: with_alpha(self.skin.hud, 0.85),
                    align: Align::Left,
                },
            );
        }

        let rank_column = card_height * BOARD_RANK_COLUMN;
        let rank_colour = match row.place {
            0..=2 => self.skin.podium[row.place],
            _ => lighten(colour, BOARD_RANK_LIFT),
        };
        font.draw(
            pixmap,
            Label {
                text: &format!("{}", row.place + 1),
                x: left + width - card_height * 0.18,
                y: baseline + size * 0.62,
                size: size * 1.75,
                colour: with_alpha(rank_colour, 0.95 * presence),
                align: Align::Right,
            },
        );

        let text_x = face_x + face + card_height * 0.2;
        let text_room = left + width - rank_column - text_x;
        font.draw(
            pixmap,
            Label {
                text: &row.entry.name,
                x: text_x,
                y: baseline,
                size: name_size(&row.entry.name, font, size),
                colour: with_alpha(colour, 0.95 * presence),
                align: Align::Left,
            },
        );
        let under = compact(row.entry.score);

        let mut under_size = size * 0.78;
        let measured = font.width(&under, under_size);
        if measured > text_room && measured > 0.0 {
            under_size *= text_room / measured;
        }
        font.draw(
            pixmap,
            Label {
                text: &under,
                x: text_x,
                y: baseline + size * 1.05,
                size: under_size,
                colour: with_alpha(darken(colour, 0.22), 0.9 * presence),
                align: Align::Left,
            },
        );
    }
}
