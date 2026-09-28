use super::*;

use tiny_skia::{Pixmap, Transform};

use crate::elements::Element;
use crate::layout::Layout;
use crate::skin::{blend, with_alpha};
use crate::text::{Align, Label};

const KEY_NAMES: [&str; 4] = ["K1", "K2", "M1", "M2"];

#[derive(Debug, Default)]
pub(super) struct KeyTrack {
    holds: [Vec<(f64, f64)>; 4],
    counted: [Vec<f64>; 4],
}

impl KeyTrack {
    fn pressed(&self, key: usize, time_ms: f64, rate: f64) -> f32 {
        let (down_ms, up_ms) = (KEYS_PRESS_DOWN_MS * rate, KEYS_PRESS_UP_MS * rate);
        let Some((down, up)) = self.last_hold(key, time_ms) else {
            return 0.0;
        };
        let fell = |elapsed: f64, over: f64| ((elapsed / over.max(1e-6)).clamp(0.0, 1.0)) as f32;
        if time_ms < up {
            return eased_out(fell(time_ms - down, down_ms));
        }
        let reached = eased_out(fell(up - down, down_ms));
        reached * (1.0 - eased_out(fell(time_ms - up, up_ms)))
    }

    fn lit(&self, key: usize, time_ms: f64, rate: f64) -> f32 {
        let Some((_, up)) = self.last_hold(key, time_ms) else {
            return 0.0;
        };
        if time_ms < up {
            return 1.0;
        }
        (1.0 - (time_ms - up) / (KEYS_LIT_FADE_MS * rate)).clamp(0.0, 1.0) as f32
    }

    fn last_hold(&self, key: usize, time_ms: f64) -> Option<(f64, f64)> {
        let holds = &self.holds[key];
        let index = holds.partition_point(|(from, _)| *from <= time_ms);
        index.checked_sub(1).and_then(|i| holds.get(i)).copied()
    }

    pub(super) fn build(
        cursor: &dossier_sim::CursorTrack,
        lazer: bool,
        quiet: &[(f64, f64)],
    ) -> Self {
        let holds = cursor.holds_each(lazer);
        let counted = std::array::from_fn(|key| {
            holds[key]
                .iter()
                .map(|&(down, _)| down)
                .filter(|down| !quiet.iter().any(|&(from, to)| *down >= from && *down <= to))
                .collect()
        });
        Self { holds, counted }
    }

    pub(super) fn quiet_spans(state: &dossier_sim::GameState) -> Vec<(f64, f64)> {
        let timeline = state.timeline();
        if state.is_lazer() {
            return Vec::new();
        }
        let (Some(first), Some(last)) = (
            timeline.objects.first().map(|object| object.start_ms),
            timeline
                .objects
                .iter()
                .map(|object| object.end_ms)
                .max_by(f64::total_cmp),
        ) else {
            return Vec::new();
        };
        let mut quiet = vec![
            (f64::NEG_INFINITY, first - timeline.difficulty.preempt_ms()),
            (last + timeline.difficulty.hit_window_50(), f64::INFINITY),
        ];
        quiet.extend(timeline.breaks.iter().copied());
        quiet
    }

    fn is_empty(&self) -> bool {
        self.holds.iter().all(Vec::is_empty)
    }

    fn count(&self, key: usize, time_ms: f64) -> usize {
        self.counted[key].partition_point(|down| *down <= time_ms)
    }

    fn named(&self, key: usize, time_ms: f64, rate: f64) -> f32 {
        let Some(&(first, _)) = self.holds[key].first() else {
            return 1.0;
        };
        if time_ms < first {
            return 1.0;
        }
        (1.0 - (time_ms - first) / (KEYS_SWAP_MS * rate)).clamp(0.0, 1.0) as f32
    }
}

const KEYS_PRESS_DOWN_MS: f64 = 160.0;
const KEYS_PRESS_UP_MS: f64 = 160.0;
const KEYS_LIT_FADE_MS: f64 = 100.0;
const KEYS_SWAP_MS: f64 = 100.0;

const OWN_ART_PER: f32 = 3.0;
const OWN_KEY: (f32, f32) = (43.0, 46.0);
const OWN_PLATE: (f32, f32) = (193.0, 55.0);
const OWN_KEY_CORNER: f32 = 9.0;
const OWN_KEY_EDGE: f32 = 2.0;
const OWN_KEY_FILL: f32 = 0.18;
const OWN_KEY_RIM: f32 = 0.8;
const OWN_PLATE_FILL: f32 = 0.5;
const OWN_PLATE_RIM: f32 = 0.12;

#[derive(Debug)]
pub(super) struct OverlayArt {
    key: Pixmap,
    plate: Pixmap,
}

impl OverlayArt {
    pub(super) fn drawn() -> Option<Self> {
        let at = |(wide, tall): (f32, f32)| {
            Pixmap::new(
                (wide * OWN_ART_PER).round() as u32,
                (tall * OWN_ART_PER).round() as u32,
            )
        };
        let mut key = at(OWN_KEY)?;
        let edge = OWN_KEY_EDGE * OWN_ART_PER;
        let side = key.width() as f32 - edge;
        let cap = rounded_rect(
            edge / 2.0,
            (key.height() as f32 - side) / 2.0,
            side,
            side,
            OWN_KEY_CORNER * OWN_ART_PER,
        )?;
        fill_and_edge(
            &mut key,
            &cap,
            with_alpha(tiny_skia::Color::WHITE, OWN_KEY_FILL),
            with_alpha(tiny_skia::Color::WHITE, OWN_KEY_RIM),
            edge,
        );

        let mut plate = at(OWN_PLATE)?;
        let rim = OWN_ART_PER;
        let bar = rounded_rect(
            rim / 2.0,
            rim / 2.0,
            plate.width() as f32 - rim,
            plate.height() as f32 - rim,
            plate.height() as f32 * 0.3,
        )?;
        fill_and_edge(
            &mut plate,
            &bar,
            with_alpha(tiny_skia::Color::BLACK, OWN_PLATE_FILL),
            with_alpha(tiny_skia::Color::WHITE, OWN_PLATE_RIM),
            rim,
        );
        Some(Self { key, plate })
    }
}

fn fill_and_edge(
    pixmap: &mut Pixmap,
    path: &tiny_skia::Path,
    fill: tiny_skia::Color,
    rim: tiny_skia::Color,
    width: f32,
) {
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(fill);
    pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
    paint.set_color(rim);
    pixmap.stroke_path(
        path,
        &paint,
        &Stroke {
            width,
            ..Default::default()
        },
        Transform::identity(),
        None,
    );
}

impl Scene<'_> {
    pub(super) fn draw_keys(
        &self,
        pixmap: &mut Pixmap,
        time_ms: f64,
        layout: &Layout,
        presence: f32,
    ) {
        if presence <= 0.01 || !self.skin.keypad || self.keys.is_empty() {
            return;
        }
        self.draw_overlay_keys(pixmap, time_ms, layout, presence);
    }

    fn overlay_art(&self, element: Element) -> Option<(&Pixmap, f32)> {
        if let Some(sprites) = self.skin.sprites.as_ref() {
            if !sprites.draw_ourselves(element) {
                return sprites.coloured(element, 0);
            }
        }
        let own = self.overlay_art.as_ref()?;
        match element {
            Element::InputOverlayKey => Some((&own.key, OWN_ART_PER)),
            Element::InputOverlayBackground => Some((&own.plate, OWN_ART_PER)),
            _ => None,
        }
    }
}

const OVERLAY_PLATE_TOP: f32 = 320.0;
const OVERLAY_STRETCH: f32 = 1.05;
const OVERLAY_KEY_FROM_RIGHT: f32 = 24.0;
const OVERLAY_KEY_TOP: f32 = 350.4;
const OVERLAY_KEY_STEP: f32 = 47.2;
const OVERLAY_PRESSED: f32 = 0.8;
const OVERLAY_TEXT: f32 = 14.0;

impl Scene<'_> {
    fn draw_overlay_keys(
        &self,
        pixmap: &mut Pixmap,
        time_ms: f64,
        layout: &Layout,
        presence: f32,
    ) {
        let right = layout.width as f32;
        let ink = self.overlay_ink();

        let (plate, length) = self.plate_size(layout);
        if length > 0.0 {
            self.draw_upright(
                pixmap,
                Element::InputOverlayBackground,
                (
                    right - plate,
                    self.skin_pixels(layout, OVERLAY_PLATE_TOP),
                    plate,
                    length,
                ),
                presence,
            );
        }

        let (key_wide, key_tall) = self.key_size(layout);
        let centre_x = right - self.skin_pixels(layout, OVERLAY_KEY_FROM_RIGHT);
        let rate = self.state.playback_rate().max(0.001);
        for index in 0..KEY_NAMES.len() {
            let squeeze = 1.0 + (OVERLAY_PRESSED - 1.0) * self.keys.pressed(index, time_ms, rate);
            let centre_y = self.skin_pixels(
                layout,
                OVERLAY_KEY_TOP + OVERLAY_KEY_STEP * index as f32,
            );
            let (wide, tall) = (key_wide * squeeze, key_tall * squeeze);
            let lit = blend(
                tiny_skia::Color::WHITE,
                active_colour(index),
                self.keys.lit(index, time_ms, rate),
            );
            self.draw_key_sprite(
                pixmap,
                (centre_x - wide / 2.0, centre_y - tall / 2.0),
                wide,
                lit,
                presence,
            );

            let shown = 1.0 - self.keys.named(index, time_ms, rate);
            if shown > 0.0 {
                let text = self.skin_pixels(layout, OVERLAY_TEXT) * squeeze;
                let count = self.keys.count(index, time_ms).to_string();
                self.draw_key_text(
                    pixmap,
                    &count,
                    (centre_x, centre_y + text * 0.5),
                    text,
                    ink,
                    presence * shown,
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
    ) {
        if alpha <= 0.01 {
            return;
        }
        if self.draw_hud_text_in(pixmap, text, x, baseline, size, Align::Centre, alpha, ink) {
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
        let Some((art, per)) = self.overlay_art(Element::InputOverlayKey) else {
            return (0.0, 0.0);
        };
        (
            self.skin_pixels(layout, art.width() as f32 / per),
            self.skin_pixels(layout, art.height() as f32 / per),
        )
    }

    fn plate_size(&self, layout: &Layout) -> (f32, f32) {
        let Some((art, per)) = self.overlay_art(Element::InputOverlayBackground) else {
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
        let Some((art, _)) = self.overlay_art(element) else {
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
        wide: f32,
        colour: tiny_skia::Color,
        alpha: f32,
    ) {
        let Some((art, _)) = self.overlay_art(Element::InputOverlayKey) else {
            return;
        };
        if alpha <= 0.0 || wide <= 0.0 {
            return;
        }
        let painted = crate::imported::tinted(art, colour);

        let scale = wide / art.width() as f32;
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
        KeyTrack::build(&dossier_sim::CursorTrack::new(frames), false, &[])
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
    fn a_press_lights_the_key_at_once_and_the_light_goes_over_a_hundred() {
        let track = track(&[(0, 0), (100, Keys::K1), (400, 0)]);
        let at = |t: f64| track.lit(0, t, 1.0);
        assert_eq!(at(99.0), 0.0);
        assert_eq!(at(100.0), 1.0, "lit on the press itself");
        assert_eq!(at(399.0), 1.0);
        assert!((at(450.0) - 0.5).abs() < 1e-6, "halfway out at fifty");
        assert_eq!(at(500.0), 0.0);
    }

    #[test]
    fn a_press_in_a_quiet_span_is_not_counted_but_still_shows_the_count() {
        let frames = [(0, 0), (100, Keys::K1), (150, 0), (1_000, Keys::K1), (1_050, 0)]
            .iter()
            .map(|&(time_ms, keys)| ReplayFrame {
                time_ms,
                x: 0.0,
                y: 0.0,
                keys: Keys(keys),
            })
            .collect();
        let track = KeyTrack::build(
            &dossier_sim::CursorTrack::new(frames),
            false,
            &[(f64::NEG_INFINITY, 500.0)],
        );
        assert_eq!(track.count(0, 400.0), 0, "before the map starts");
        assert_eq!(track.named(0, 400.0, 1.0), 0.0, "but the zero is up");
        assert_eq!(track.count(0, 1_100.0), 1);
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
