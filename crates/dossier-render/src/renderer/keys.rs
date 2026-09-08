use super::*;

use tiny_skia::{Pixmap, Transform};

use crate::elements::Element;
use crate::layout::Layout;
use crate::skin::{blend, with_alpha};
use crate::text::{Align, Label};

fn ease_out(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    1.0 - (1.0 - x) * (1.0 - x)
}

const KEY_NAMES: [&str; 4] = ["K1", "K2", "M1", "M2"];

#[derive(Debug, Default)]
pub(super) struct KeyTrack {
    holds: [Vec<(f64, f64)>; 4],
}

impl KeyTrack {
    fn pressed(&self, key: usize, time_ms: f64, rate: f64) -> f32 {
        let (down_ms, up_ms) = (KEYS_PRESS_DOWN_MS * rate, KEYS_PRESS_UP_MS * rate);
        let holds = &self.holds[key];
        let index = holds.partition_point(|(from, _)| *from <= time_ms);
        let Some(&(down, up)) = index.checked_sub(1).and_then(|i| holds.get(i)) else {
            return 0.0;
        };
        let fell = |elapsed: f64, over: f64| ((elapsed / over.max(1e-6)).clamp(0.0, 1.0)) as f32;
        if time_ms < up {
            return ease_out(fell(time_ms - down, down_ms));
        }
        let reached = ease_out(fell(up - down, down_ms));
        reached * (1.0 - ease_out(fell(time_ms - up, up_ms)))
    }

    pub(super) fn build(cursor: &dossier_sim::CursorTrack, lazer: bool) -> Self {
        Self {
            holds: cursor.holds_each(lazer),
        }
    }

    fn count(&self, key: usize, time_ms: f64) -> usize {
        self.holds[key].partition_point(|(from, _)| *from <= time_ms)
    }

    fn named(&self, key: usize, time_ms: f64, rate: f64) -> f32 {
        let Some(&(first, _)) = self.holds[key].first() else {
            return 1.0;
        };
        if time_ms < first {
            return 1.0;
        }
        1.0 - ease_out(((time_ms - first) / (KEYS_SWAP_MS * rate)) as f32)
    }
}

const KEYS_INSET: f64 = 0.018;

const KEYS_BOX: f64 = 0.052;

const KEYS_WIDTH: f32 = 1.35;

const KEYS_GAP: f32 = 0.18;

const KEYS_TRAIL_MS: f64 = 1_300.0;

const KEYS_TRAIL_REACH: f64 = 0.135;

const KEYS_MARK_MIN: f32 = 0.03;

const KEYS_MARK_HEIGHT: f32 = 0.6;

const KEYS_PRESS_SHRINK: f32 = 0.14;

const KEYS_PRESS_DOWN_MS: f64 = 160.0;
const KEYS_PRESS_UP_MS: f64 = 160.0;

impl Scene<'_> {
    fn draw_key_trail(
        &self,
        pixmap: &mut Pixmap,
        key: usize,
        time_ms: f64,
        layout: &Layout,
        presence: f32,
        place: (f32, f32, f32),
    ) {
        let (right, top, height) = place;
        let reach = (f64::from(layout.width) * KEYS_TRAIL_REACH) as f32;
        let rate = self.state.playback_rate().max(0.001);

        let window = KEYS_TRAIL_MS * rate;
        let from = time_ms - window;

        let bar = height * KEYS_MARK_HEIGHT;
        let bar_top = top + (height - bar) / 2.0;
        let x_of = |at: f64| right - ((time_ms - at) / window) as f32 * reach;

        for &(down, up) in self.keys.holds[key]
            .iter()
            .rev()
            .take_while(|(_, up)| *up >= from)
        {
            let (a, b) = (down.max(from), up.min(time_ms));
            if b <= a {
                continue;
            }
            let (left, width) = (x_of(a), (x_of(b) - x_of(a)).max(1.0));

            let width = width.max(reach * KEYS_MARK_MIN);
            let Some(mark) = rounded_rect(left, bar_top, width, bar, bar * 0.35) else {
                continue;
            };

            let age = ((time_ms - b) / window).clamp(0.0, 1.0) as f32;
            let mut paint = Paint {
                anti_alias: true,
                ..Default::default()
            };
            paint.set_color(with_alpha(
                self.skin.verdict_miss,
                0.75 * (1.0 - age) * presence,
            ));
            pixmap.fill_path(
                &mark,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    pub(super) fn draw_keys(
        &self,
        pixmap: &mut Pixmap,
        time_ms: f64,
        layout: &Layout,
        presence: f32,
    ) {
        if presence <= 0.01 || !self.skin.keypad {
            return;
        }
        if self.skin_speaks_for(Element::InputOverlayKey) {
            self.draw_skin_keys(pixmap, time_ms, layout, presence);
            return;
        }
        let Some(font) = &self.skin.font else {
            return;
        };
        let (width, height) = (f64::from(layout.width), f64::from(layout.height));
        let box_side = (height * KEYS_BOX) as f32;
        let box_wide = box_side * KEYS_WIDTH;
        let step = box_side * (1.0 + KEYS_GAP);
        let right = (width * (1.0 - KEYS_INSET)) as f32;

        let top = (height as f32 - (step * 4.0 - box_side * KEYS_GAP)) / 2.0;

        let rate = self.state.playback_rate().max(0.001);
        for (index, name) in KEY_NAMES.iter().enumerate() {
            let down = self.keys.pressed(index, time_ms, rate);
            if self.skin.key_bars {
                self.draw_key_trail(pixmap, index, time_ms, layout, presence, {
                    let y = top + step * index as f32;
                    (right - box_wide, y, box_side)
                });
            }
            let count = self.keys.count(index, time_ms);
            let shrink = KEYS_PRESS_SHRINK * down;
            let side = box_side * (1.0 - shrink);
            let wide = box_wide * (1.0 - shrink);
            let x = right - box_wide + (box_wide - wide) / 2.0;
            let y = top + step * index as f32 + (box_side - side) / 2.0;

            let Some(card) = rounded_rect(x, y, wide, side, side * 0.3) else {
                continue;
            };
            let mut fill = Paint {
                anti_alias: true,
                ..Default::default()
            };

            let body = with_alpha(
                blend(self.skin.background, self.skin.verdict_miss, down),
                (0.55 + 0.30 * down) * presence,
            );
            let ink = self.skin.hud;
            fill.set_color(body);
            pixmap.fill_path(&card, &fill, FillRule::Winding, Transform::identity(), None);

            let mut edge = Paint {
                anti_alias: true,
                ..Default::default()
            };
            edge.set_color(with_alpha(ink, (0.35 + 0.55 * down) * presence));
            pixmap.stroke_path(
                &card,
                &edge,
                &Stroke {
                    width: (side * 0.05).max(1.0),
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );

            font.draw(
                pixmap,
                Label {
                    text: name,
                    x: x + wide / 2.0,
                    y: y + side * 0.34,
                    size: side * 0.26,
                    colour: with_alpha(ink, 0.7 * presence),
                    align: Align::Centre,
                },
            );

            let count = count.to_string();
            if !self.draw_hud_text(
                pixmap,
                &count,
                x + wide / 2.0,
                y + side * 0.78,
                side * 0.42,
                Align::Centre,
                0.95 * presence,
            ) {
                font.draw(
                    pixmap,
                    Label {
                        text: &count,
                        x: x + wide / 2.0,
                        y: y + side * 0.78,
                        size: side * 0.42,
                        colour: with_alpha(ink, 0.95 * presence),
                        align: Align::Centre,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod keys {
    use super::KeyTrack;
    use dossier_replay::{Keys, ReplayFrame};

    fn track(script: &[(i64, u8)]) -> KeyTrack {
        let frames = script
            .iter()
            .map(|&(time_ms, keys)| ReplayFrame {
                time_ms,
                x: 0.0,
                y: 0.0,
                keys: Keys(keys),
            })
            .collect();
        KeyTrack::build(&dossier_sim::CursorTrack::new(frames), false)
    }

    #[test]
    fn a_keyboard_press_is_one_press_and_not_two() {
        let track = track(&[
            (0, 0),
            (10, Keys::M1 | Keys::K1),
            (20, 0),
            (30, Keys::M1 | Keys::K1),
            (40, 0),
            (50, Keys::M1 | Keys::K1),
            (60, 0),
        ]);
        assert_eq!(track.count(0, 100.0), 3);
    }

    #[test]
    fn a_mouse_press_lands_in_the_mouse_button() {
        let track = track(&[(0, 0), (10, Keys::M1), (20, 0), (30, Keys::M2), (40, 0)]);
        assert_eq!(track.count(0, 100.0), 0, "K1");
        assert_eq!(track.count(1, 100.0), 0, "K2");
        assert_eq!(track.count(2, 100.0), 1, "M1");
        assert_eq!(track.count(3, 100.0), 1, "M2");
    }

    #[test]
    fn a_keyboard_press_is_not_counted_as_a_mouse_press_as_well() {
        let track = track(&[
            (0, 0),
            (10, Keys::K1 | Keys::M1),
            (20, 0),
            (30, Keys::K2 | Keys::M2),
            (40, 0),
        ]);
        assert_eq!(track.count(0, 100.0), 1, "K1");
        assert_eq!(track.count(1, 100.0), 1, "K2");
        assert_eq!(
            track.count(2, 100.0),
            0,
            "M1 — the bit was set, the press was not"
        );
        assert_eq!(track.count(3, 100.0), 0, "M2");
    }

    #[test]
    fn a_count_is_as_of_the_instant_asked_for() {
        let track = track(&[(0, 0), (100, Keys::K1), (150, 0), (200, Keys::K1), (250, 0)]);
        assert_eq!(track.count(0, 50.0), 0);
        assert_eq!(track.count(0, 120.0), 1);
        assert_eq!(track.count(0, 220.0), 2);
    }

    #[test]
    fn a_press_goes_down_over_time_rather_than_at_once() {
        let track = track(&[(0, 0), (100, Keys::K1), (400, 0)]);
        let at = |t: f64| track.pressed(0, t, 1.0);

        assert_eq!(at(99.0), 0.0, "nothing before the press");
        assert!(at(100.0) < 0.05, "the press starts at the top");
        assert!(at(115.0) > at(105.0), "and travels");
        assert!(at(260.0) > 0.99, "arriving a hundred and sixty later");
        assert!(at(390.0) > 0.99, "and staying there");
    }

    #[test]
    fn a_release_takes_as_long_as_the_press_did() {
        let track = track(&[(0, 0), (100, Keys::K1), (400, 0)]);
        let at = |t: f64| track.pressed(0, t, 1.0);
        assert!(at(430.0) > 0.2, "still visibly down a frame or two after");
        assert!(at(561.0) < 0.01, "and at rest a hundred and sixty later");

        let gone_down = at(180.0);
        let come_up = 1.0 - at(480.0);
        assert!(
            (come_up - gone_down).abs() < 0.01,
            "the release ({come_up:.3}) went at another pace than the press ({gone_down:.3})"
        );
    }

    #[test]
    fn a_tap_too_short_to_land_starts_back_from_where_it_reached() {
        let track = track(&[(0, 0), (100, Keys::K1), (110, 0)]);
        let at = |t: f64| track.pressed(0, t, 1.0);
        let peak = at(110.0);
        assert!(peak > 0.0 && peak < 0.7, "a 10ms tap reached {peak}");
        assert!(at(140.0) < peak, "and comes back from there");
    }

    #[test]
    fn the_animation_keeps_its_pace_under_a_rate_mod() {
        let track = track(&[(0, 0), (100, Keys::K1), (900, 0)]);
        let plain = track.pressed(0, 130.0, 1.0);

        let fast = track.pressed(0, 100.0 + 30.0 * 1.5, 1.5);
        assert!((plain - fast).abs() < 1e-6, "{plain} against {fast}");
    }

    #[test]
    fn a_button_still_down_at_the_end_still_counts() {
        let track = track(&[(0, 0), (100, Keys::K2)]);
        assert_eq!(track.count(1, 200.0), 1);

        assert!(track.pressed(1, 100.5, 1.0) > 0.0);
    }

    #[test]
    fn a_key_wears_its_name_until_it_is_first_pressed() {
        let track = track(&[(0, 0), (500, Keys::K1), (600, 0)]);
        assert_eq!(track.named(0, 100.0, 1.0), 1.0, "before any press");
        assert_eq!(track.named(0, 500.0, 1.0), 1.0, "at the press itself");
        assert!(track.named(0, 580.0, 1.0) < 1.0, "and gives way");
        assert_eq!(track.named(0, 700.0, 1.0), 0.0, "to the count");
        assert_eq!(
            track.named(1, 10_000.0, 1.0),
            1.0,
            "a key never pressed keeps its name"
        );
    }
}

const OVERLAY_KEY: f32 = 46.0;
const OVERLAY_SPACING: f32 = 1.8;
const OVERLAY_PRESSED: f32 = 0.75;

const OVERLAY_KEY_INSET: f32 = 1.5;
const OVERLAY_KEY_DROP: f32 = 7.0;

const KEYS_SWAP_MS: f64 = 160.0;

const OVERLAY_PLATE_RISE: f32 = 64.0;

const OVERLAY_STRETCH: f32 = 1.05;

const OVERLAY_TEXT: f32 = 0.32;

impl Scene<'_> {
    fn draw_skin_keys(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, presence: f32) {
        let (key, key_tall) = self.key_size(layout);
        let gap = self.skin_pixels(layout, OVERLAY_SPACING);
        let right = layout.width as f32;
        let ink = self.overlay_ink();

        let (plate, length) = self.plate_size(layout);
        let drop = self.skin_pixels(layout, OVERLAY_KEY_DROP);
        let plate_top = layout.height as f32 / 2.0 - self.skin_pixels(layout, OVERLAY_PLATE_RISE);
        if length > 0.0 {
            self.draw_upright(
                pixmap,
                Element::InputOverlayBackground,
                (right - plate, plate_top, plate, length),
                presence,
            );
        }
        let top = plate_top + drop;

        let rate = self.state.playback_rate().max(0.001);
        for (index, name) in KEY_NAMES.iter().enumerate() {
            let down = self.keys.pressed(index, time_ms, rate);

            let shrink = 1.0 + (OVERLAY_PRESSED - 1.0) * down;
            let side = key * shrink;
            let wall = right - self.skin_pixels(layout, OVERLAY_KEY_INSET);
            let centre_x = wall - key / 2.0;
            let centre_y = top + (key_tall + gap) * index as f32 + key_tall / 2.0;

            let lit = blend(tiny_skia::Color::WHITE, active_colour(index), down);
            self.draw_key_sprite(
                pixmap,
                (centre_x - side / 2.0, centre_y - side / 2.0),
                side,
                lit,
                presence,
            );

            let named = self.keys.named(index, time_ms, rate);
            let text = key * OVERLAY_TEXT * shrink;
            let count_x = centre_x + self.key_count_offset(side);
            if named < 1.0 {
                let count = self.keys.count(index, time_ms).to_string();
                self.draw_key_text(
                    pixmap,
                    &count,
                    (count_x, centre_y + text * 0.5),
                    text,
                    ink,
                    presence * (1.0 - named),
                    true,
                );
            }
            if named > 0.0 {
                self.draw_key_text(
                    pixmap,
                    name,
                    (centre_x, centre_y + text * 0.5),
                    text,
                    ink,
                    presence * named,
                    false,
                );
            }
        }
    }

    fn overlay_ink(&self) -> tiny_skia::Color {
        self.skin
            .sprites
            .as_ref()
            .and_then(|s| s.ini().input_overlay_text)
            .unwrap_or(self.skin.hud)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_key_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        (x, baseline): (f32, f32),
        size: f32,
        ink: tiny_skia::Color,
        alpha: f32,
        glyphs: bool,
    ) {
        if alpha <= 0.01 {
            return;
        }
        if glyphs
            && self.draw_hud_text_in(pixmap, text, x, baseline, size, Align::Centre, alpha, ink)
        {
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
                colour: with_alpha(ink, alpha),
                align: Align::Centre,
            },
        );
    }

    fn key_size(&self, layout: &Layout) -> (f32, f32) {
        match self.key_art() {
            Some((art, _, _, _, _)) => (
                self.skin_pixels(layout, art.width() as f32 / self.key_per()),
                self.skin_pixels(layout, art.height() as f32 / self.key_per()),
            ),
            None => {
                let side = self.skin_pixels(layout, OVERLAY_KEY);
                (side, side)
            }
        }
    }

    fn key_per(&self) -> f32 {
        self.skin
            .sprites
            .as_ref()
            .and_then(|s| s.coloured(Element::InputOverlayKey, 0))
            .map_or(1.0, |(_, per)| per)
    }

    fn key_count_offset(&self, wide: f32) -> f32 {
        match self.key_art() {
            Some((art, _, _, left, _)) if left > 1.0 => {
                let share = left / art.width() as f32;
                wide * (share / 2.0 - 0.5)
            }
            _ => 0.0,
        }
    }

    fn key_art(&self) -> Option<(&tiny_skia::Pixmap, f32, f32, f32, f32)> {
        let (art, per) = self
            .skin
            .sprites
            .as_ref()?
            .coloured(Element::InputOverlayKey, 0)?;
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
        for (index, pixel) in art.pixels().iter().enumerate() {
            if pixel.alpha() == 0 {
                continue;
            }
            let (x, y) = (index as u32 % art.width(), index as u32 / art.width());
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        let _ = per;
        Some((
            art,
            (x1 - x0) as f32,
            (y1 - y0) as f32,
            x0 as f32,
            y0 as f32,
        ))
    }

    fn plate_size(&self, layout: &Layout) -> (f32, f32) {
        let Some(sprites) = self.skin.sprites.as_ref() else {
            return (0.0, 0.0);
        };
        let Some((art, per)) = sprites.coloured(Element::InputOverlayBackground, 0) else {
            return (0.0, 0.0);
        };

        (
            self.skin_pixels(layout, art.height() as f32 / per),
            self.skin_pixels(layout, art.width() as f32 / per) * OVERLAY_STRETCH,
        )
    }

    fn draw_upright(
        &self,
        pixmap: &mut Pixmap,
        element: Element,
        (x, y, wide, tall): (f32, f32, f32, f32),
        alpha: f32,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };
        let Some((art, _)) = sprites.coloured(element, 0) else {
            return;
        };
        if alpha <= 0.0 || wide <= 0.0 || tall <= 0.0 {
            return;
        }

        let transform = Transform::from_translate(x + wide, y)
            .pre_rotate(90.0)
            .pre_scale(tall / art.width() as f32, wide / art.height() as f32);
        pixmap.draw_pixmap(
            0,
            0,
            art.as_ref(),
            &tiny_skia::PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            transform,
            None,
        );
    }

    fn draw_key_sprite(
        &self,
        pixmap: &mut Pixmap,
        (x, y): (f32, f32),
        side: f32,
        colour: tiny_skia::Color,
        alpha: f32,
    ) {
        let Some(sprites) = &self.skin.sprites else {
            return;
        };
        let Some((art, _)) = sprites.coloured(Element::InputOverlayKey, 0) else {
            return;
        };
        if alpha <= 0.0 || side <= 0.0 {
            return;
        }
        let painted = crate::imported::tinted(art, colour);

        let scale = side / art.width() as f32;
        pixmap.draw_pixmap(
            0,
            0,
            painted.as_ref(),
            &tiny_skia::PixmapPaint {
                opacity: alpha.clamp(0.0, 1.0),
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            Transform::from_translate(x, y).pre_scale(scale, scale),
            None,
        );
    }
}

fn active_colour(key: usize) -> tiny_skia::Color {
    if key < 2 {
        tiny_skia::Color::from_rgba8(0xff, 0xde, 0x00, 0xff)
    } else {
        tiny_skia::Color::from_rgba8(0xf8, 0x00, 0x9e, 0xff)
    }
}
