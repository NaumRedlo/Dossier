use super::format::{compact, name_size};
use super::paint::rounded_rect;
use super::*;

use tiny_skia::{Color, FillRule, Paint, Pixmap, Transform};

use crate::layout::Layout;
use crate::skin::{darken, lighten, with_alpha};
use crate::text::{Align, Label};

impl Scene<'_> {
    pub(super) fn draw_leaderboard(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        let (Some(font), false) = (&self.skin.font, self.leaderboard.is_empty()) else {
            return;
        };
        let Some(track) = self.state.score_track() else {
            return;
        };
        let rows = self
            .leaderboard
            .standings_at(&ScoreCurve(track), time_ms, BOARD_ROWS);

        let height = f64::from(layout.height);
        let size = (height * BOARD_TEXT) as f32;
        let step = (height * BOARD_STEP) as f32;
        let left = (height * BOARD_LEFT) as f32;
        let width = (height * BOARD_WIDTH) as f32;
        let card_height = step * BOARD_CARD_FILL;

        let drawn = BOARD_ROWS as f32;
        let top = pixmap.height() as f32 / 2.0 + (drawn / 2.0 - 1.0) * step;

        for row in &rows {
            let eased = {
                let t = row.moving.clamp(0.0, 1.0);
                1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
            };

            let slot = row.from_slot + (row.slot - row.from_slot) * eased;
            let y = top - slot * step + size * 1.15;

            let t = row.moving.clamp(0.0, 1.0);

            let late = 1.0 - t * t * t;
            let settling = if row.leaving {
                late
            } else if row.entering {
                eased
            } else if (row.slot - row.from_slot).abs() < f32::EPSILON {
                1.0
            } else {
                eased
            };
            let presence = if row.leaving || row.entering {
                settling
            } else {
                0.45 + 0.55 * settling
            };
            let shrink = if row.leaving || row.entering {
                BOARD_GROW + (1.0 - BOARD_GROW) * settling
            } else {
                0.94 + 0.06 * settling
            };

            let card_w = width * shrink;
            let card_h = card_height * shrink;
            self.draw_board_row(
                pixmap,
                font,
                row,
                left + (width - card_w) / 2.0,
                y - (card_height - card_h) / 2.0,
                card_w,
                card_h,
                size * shrink,
                presence,
            );
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
        presence: f32,
    ) {
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
        }

        for (grow, alpha) in [(BOARD_GLOW, 0.22), (0.0, 0.95)] {
            let Some(ring) = rounded_rect(
                face_x - face * grow,
                face_y - face * grow,
                face * (1.0 + grow * 2.0),
                face * (1.0 + grow * 2.0),
                face * 0.28,
            ) else {
                continue;
            };
            let mut edge = Paint {
                anti_alias: true,
                ..Default::default()
            };
            edge.set_color(with_alpha(self.skin.verdict_miss, alpha * presence));
            pixmap.stroke_path(
                &ring,
                &edge,
                &Stroke {
                    width: face * BOARD_RING,
                    ..Default::default()
                },
                Transform::identity(),
                None,
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
        let mut under = compact(row.entry.score);
        if let Some(accuracy) = row.entry.accuracy {
            under.push_str(&format!("  {accuracy:.2}%"));
        }
        if !row.entry.mods.is_empty() {
            under.push_str(&format!("  {}", row.entry.mods));
        }

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

struct ScoreCurve<'a>(&'a dossier_sim::ScoreTrack);

impl crate::leaderboard::ScoreAt for ScoreCurve<'_> {
    fn at(&self, time_ms: f64) -> u64 {
        self.0.at(time_ms)
    }

    fn reached(&self, score: u64) -> f64 {
        self.0.reached(score)
    }
}
