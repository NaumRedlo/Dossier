pub mod draw;
pub mod context;
pub mod hold;
pub mod node;
mod plates;
pub mod sample;

use std::time::Instant;

use dossier_overlay::{Screen, View, Viewport};
use serde::{Deserialize, Serialize};

pub use node::Node;

pub const REFERENCE_HIGH: f32 = 720.0;
pub const EDGE: f32 = 24.0;
pub const SIZE: f32 = 0.86;
pub const UNFOLD_SECONDS: f32 = 0.22;
pub const RATE_HALF_LIFE: f32 = 0.12;
pub const PULSE_SECONDS: f32 = 1.6;
pub const METER_HIGH: f32 = 30.0;
pub const HOLD_SECONDS: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    #[default]
    Ru,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Good,
    Blue,
    Accent,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Best {
    pub title: String,
    pub pp: f64,
    pub accuracy: f64,
    pub mods: String,
    pub stars: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Day {
    pub pp_gain: f64,
    pub rank: u32,
    pub climbed: i64,
    pub best: Option<Best>,
    pub plays: u32,
    pub seconds: u64,
    pub records: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Place {
    pub place: u32,
    pub name: String,
    pub accuracy: f64,
    pub you: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Card {
    pub pool: Option<String>,
    pub stars: Option<f64>,
    pub places: Vec<Place>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Meter {
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Play {
    pub pp: Option<f64>,
    pub record: Option<f64>,
    pub clean: Option<f64>,
    pub resting: bool,
    pub unstable_rate: Option<f64>,
    pub meter: Option<Meter>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sent {
    pub chat: String,
    pub title: String,
    pub player: String,
    pub mods: String,
    pub pp: f64,
    pub accuracy: f64,
    pub grade: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Outcome {
    pub saved: bool,
    pub offer: Option<String>,
    pub holding: f32,
    pub sent: Option<Sent>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Notice {
    pub title: String,
    pub body: String,
    pub sign: String,
    pub tone: Tone,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Context {
    pub lang: Lang,
    pub day: Option<Day>,
    pub card: Option<Card>,
    pub play: Option<Play>,
    pub outcome: Option<Outcome>,
    pub notices: Vec<Notice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stage {
    pub unfold: f32,
    pub rate: Option<f32>,
    pub pulse: f32,
    last: Option<Instant>,
}

pub fn told(view: &View, context: &mut Context) {
    let Some(game) = &view.snapshot.gameplay else {
        return;
    };
    let play = context.play.get_or_insert_with(Play::default);
    if let Some(resting) = game.resting {
        play.resting = resting;
    }
    play.unstable_rate = game.unstable_rate;
    play.pp = game.pp;
    if game.pp_clean.is_some() {
        play.clean = game.pp_clean;
    }
    play.meter = view
        .snapshot
        .meter
        .filter(|meter| meter.shown)
        .map(|meter| Meter { scale: meter.scale });
}

fn live(view: Option<&View>) -> bool {
    view.is_some_and(|view| {
        view.snapshot.screen == Screen::Playing && view.snapshot.watching_replay != Some(true)
    })
}

impl Stage {
    pub fn settled(view: Option<&View>, context: &Context) -> Stage {
        let resting = context.play.as_ref().is_none_or(|play| play.resting);
        Stage {
            unfold: if live(view) && !resting { 0.0 } else { 1.0 },
            rate: context
                .play
                .as_ref()
                .and_then(|play| play.unstable_rate)
                .map(|rate| rate as f32),
            pulse: 0.0,
            last: None,
        }
    }

    pub fn advance(&mut self, view: Option<&View>, context: &Context, now: Instant) {
        let dt = self
            .last
            .map_or(0.0, |was| now.saturating_duration_since(was).as_secs_f32())
            .min(0.25);
        self.last = Some(now);
        self.pulse = (self.pulse + dt) % PULSE_SECONDS;
        let resting = context.play.as_ref().is_none_or(|play| play.resting);
        let aim = if live(view) && !resting { 0.0 } else { 1.0 };
        let step = dt / UNFOLD_SECONDS;
        self.unfold = if self.unfold < aim {
            (self.unfold + step).min(aim)
        } else {
            (self.unfold - step).max(aim)
        };
        let wanted = context
            .play
            .as_ref()
            .filter(|_| live(view))
            .and_then(|play| play.unstable_rate)
            .map(|rate| rate as f32);
        self.rate = match (self.rate, wanted) {
            (Some(shown), Some(aim)) => {
                let keep = 0.5f32.powf(dt / RATE_HALF_LIFE);
                let next = aim + (shown - aim) * keep;
                Some(if (next - aim).abs() < 0.05 { aim } else { next })
            }
            (_, aim) => aim,
        };
    }

    pub fn moving(&self, view: Option<&View>, context: &Context) -> bool {
        let resting = context.play.as_ref().is_none_or(|play| play.resting);
        let aim = if live(view) && !resting { 0.0 } else { 1.0 };
        let wanted = context
            .play
            .as_ref()
            .and_then(|play| play.unstable_rate)
            .map(|rate| rate as f32);
        (self.unfold - aim).abs() > 0.001
            || live(view)
            || matches!((self.rate, wanted), (Some(shown), Some(aim)) if (shown - aim).abs() > 0.05)
    }

    pub fn eased(&self) -> f32 {
        let k = self.unfold.clamp(0.0, 1.0);
        k * k * (3.0 - 2.0 * k)
    }

    pub fn glow(&self) -> f32 {
        0.35 + 0.65 * (0.5 + 0.5 * (std::f32::consts::TAU * self.pulse / PULSE_SECONDS).cos())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    BottomCentre,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub key: &'static str,
    pub anchor: Anchor,
    pub x: f32,
    pub y: f32,
    pub node: Node,
}

fn stacked(key: [&'static str; 2], anchor: Anchor, from: f32, nodes: Vec<Node>) -> Vec<Placed> {
    let mut pen = from;
    nodes
        .into_iter()
        .zip(key)
        .map(|(node, key)| {
            let high = node.measure().1;
            let placed = Placed {
                key,
                anchor,
                x: EDGE,
                y: pen,
                node,
            };
            pen += high + 8.0;
            placed
        })
        .collect()
}

pub fn plan(view: Option<&View>, context: &Context, stage: &Stage) -> Vec<Placed> {
    let Some(view) = view else {
        return Vec::new();
    };
    let lang = context.lang;
    let screen = view.snapshot.screen;
    let playing = live(Some(view));
    let mut placed = Vec::new();
    let saved = screen == Screen::Results && context.outcome.as_ref().is_some_and(|out| out.saved);
    let tile = if playing {
        plates::tile_recording(lang, stage.eased(), stage.glow())
    } else if saved {
        plates::tile_saved(lang)
    } else {
        plates::tile_connected(lang)
    };
    let mut corner = vec![("tile", tile)];
    match screen {
        Screen::Menu => {
            if let Some(day) = &context.day {
                placed.push(Placed {
                    key: "day",
                    anchor: Anchor::TopLeft,
                    x: EDGE,
                    y: 96.0,
                    node: plates::day(day, lang),
                });
            }
            let notices: Vec<Node> = context.notices.iter().take(2).map(plates::notice).collect();
            placed.extend(stacked(
                ["notice-1", "notice-2"],
                Anchor::TopRight,
                96.0,
                notices,
            ));
        }
        Screen::Selection => {
            if let Some(node) = context
                .card
                .as_ref()
                .and_then(|card| plates::card(card, lang))
            {
                placed.push(Placed {
                    key: "card",
                    anchor: Anchor::BottomLeft,
                    x: EDGE,
                    y: EDGE,
                    node,
                });
            }
        }
        Screen::Playing if playing => {
            if let Some(play) = &context.play {
                if let Some(pp) = play.pp {
                    corner.push(("pp", plates::reach(play, pp, lang, stage.eased())));
                }
                if let (Some(meter), Some(rate)) = (play.meter, stage.rate) {
                    let node = plates::rate(rate, lang, meter.scale);
                    let wide = node.measure().0;
                    placed.push(Placed {
                        key: "rate",
                        anchor: Anchor::BottomCentre,
                        x: -wide / 2.0,
                        y: METER_HIGH / SIZE * meter.scale.clamp(0.5, 3.0) + 6.0,
                        node,
                    });
                }
            }
        }
        Screen::Results => {
            if let Some(outcome) = &context.outcome {
                if let Some(sent) = &outcome.sent {
                    corner.push(("sent", plates::sent(sent, lang)));
                } else if let Some(key) = outcome.offer.as_deref().filter(|_| outcome.saved) {
                    corner.push(("offer", plates::offer(key, outcome.holding, lang)));
                }
            }
        }
        _ => {}
    }
    let mut pen = EDGE;
    for (key, node) in corner {
        let high = node.measure().1;
        placed.push(Placed {
            key,
            anchor: Anchor::BottomRight,
            x: EDGE,
            y: pen,
            node,
        });
        pen += high + 8.0;
    }
    placed
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub key: &'static str,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub fn unit(viewport: Viewport) -> f32 {
    (viewport.height as f32 / REFERENCE_HIGH * SIZE).clamp(0.5, 4.0)
}

pub fn paint(placed: &Placed, viewport: Viewport) -> Option<Sprite> {
    let unit = unit(viewport);
    let (wide, high) = placed.node.measure();
    let room = 12.0;
    let mut canvas = draw::Canvas::new(wide + room * 2.0, high + room * 2.0, unit)?;
    placed.node.draw(&mut canvas, room, room, wide);
    let (frame_wide, frame_high) = (viewport.width as f32 / unit, viewport.height as f32 / unit);
    let (left, top) = match placed.anchor {
        Anchor::TopLeft => (placed.x, placed.y),
        Anchor::TopRight => (frame_wide - placed.x - wide, placed.y),
        Anchor::BottomLeft => (placed.x, frame_high - placed.y - high),
        Anchor::BottomRight => (frame_wide - placed.x - wide, frame_high - placed.y - high),
        Anchor::BottomCentre => (frame_wide / 2.0 + placed.x, frame_high - placed.y - high),
    };
    Some(Sprite {
        key: placed.key,
        x: ((left - room) * unit).round() as i32,
        y: ((top - room) * unit).round() as i32,
        width: canvas.pixmap.width(),
        height: canvas.pixmap.height(),
        pixels: canvas.pixmap.take(),
    })
}

pub fn sprites(
    view: Option<&View>,
    context: &Context,
    stage: &Stage,
    viewport: Viewport,
) -> Vec<Sprite> {
    plan(view, context, stage)
        .iter()
        .filter_map(|placed| paint(placed, viewport))
        .collect()
}

pub fn compose(frame: &mut [u8], viewport: Viewport, sprites: &[Sprite]) {
    let (wide, high) = (viewport.width as i32, viewport.height as i32);
    for sprite in sprites {
        for row in 0..sprite.height as i32 {
            let y = sprite.y + row;
            if y < 0 || y >= high {
                continue;
            }
            for column in 0..sprite.width as i32 {
                let x = sprite.x + column;
                if x < 0 || x >= wide {
                    continue;
                }
                let from = ((row * sprite.width as i32 + column) * 4) as usize;
                let to = ((y * wide + x) * 4) as usize;
                let alpha = u32::from(sprite.pixels[from + 3]);
                if alpha == 0 {
                    continue;
                }
                for channel in 0..3 {
                    let over = u32::from(sprite.pixels[from + channel]);
                    let under = u32::from(frame[to + channel]);
                    frame[to + channel] = (over + under * (255 - alpha) / 255).min(255) as u8;
                }
                frame[to + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn frame(name: &str) -> (View, Context) {
        sample::frames(Lang::Ru)
            .into_iter()
            .find(|(held, _, _)| *held == name)
            .map(|(_, view, context)| (view, context))
            .unwrap()
    }

    fn keys(view: &View, context: &Context) -> Vec<&'static str> {
        plan(Some(view), context, &Stage::settled(Some(view), context))
            .iter()
            .map(|placed| placed.key)
            .collect()
    }

    #[test]
    fn nothing_is_drawn_without_a_view_and_the_tile_stands_in_the_corner_on_every_screen() {
        let (_, context) = frame("menu");
        assert!(plan(None, &context, &Stage::default()).is_empty());
        for (name, view, context) in sample::frames(Lang::Ru) {
            let placed = plan(
                Some(&view),
                &context,
                &Stage::settled(Some(&view), &context),
            );
            let tile = placed
                .iter()
                .find(|placed| placed.key == "tile")
                .unwrap_or_else(|| panic!("{name} has the tile"));
            assert_eq!(
                (tile.anchor, tile.x, tile.y),
                (Anchor::BottomRight, EDGE, EDGE),
                "{name}"
            );
        }
    }

    #[test]
    fn each_screen_shows_its_own_plates() {
        let (view, context) = frame("menu");
        assert_eq!(
            keys(&view, &context),
            ["day", "notice-1", "notice-2", "tile"]
        );
        let (view, context) = frame("select");
        assert_eq!(keys(&view, &context), ["card", "tile"]);
        let (view, context) = frame("play");
        assert_eq!(keys(&view, &context), ["rate", "tile", "pp"]);
        for name in ["results", "hold"] {
            let (view, context) = frame(name);
            assert_eq!(keys(&view, &context), ["tile", "offer"], "{name}");
        }
        let (view, context) = frame("sent");
        assert_eq!(
            keys(&view, &context),
            ["tile", "sent"],
            "what was sent takes the place of the offer"
        );
    }

    #[test]
    fn the_offer_waits_for_a_saved_play_and_a_replay_is_not_a_play() {
        let (view, mut context) = frame("results");
        context.outcome.as_mut().unwrap().saved = false;
        assert_eq!(keys(&view, &context), ["tile"]);
        let (mut view, context) = frame("play");
        view.snapshot.watching_replay = Some(true);
        assert_eq!(
            keys(&view, &context),
            ["tile"],
            "a watched replay gets neither the counter nor the rate"
        );
    }

    #[test]
    fn the_offer_fills_its_bar_while_the_key_is_held_and_keeps_its_size() {
        let (view, resting) = frame("results");
        let (_, held) = frame("hold");
        let viewport = Viewport {
            width: 1280,
            height: 720,
            scale: 1.0,
        };
        let offer = |context: &Context| {
            sprites(
                Some(&view),
                context,
                &Stage::settled(Some(&view), context),
                viewport,
            )
            .into_iter()
            .find(|sprite| sprite.key == "offer")
            .unwrap()
        };
        let (still, filling) = (offer(&resting), offer(&held));
        assert_eq!(
            (still.x, still.y, still.width, still.height),
            (filling.x, filling.y, filling.width, filling.height),
            "the plate does not jump when the hold begins"
        );
        assert_ne!(still.pixels, filling.pixels);
    }

    #[test]
    fn the_counter_and_the_tile_fold_while_notes_are_played_and_unfold_in_a_break() {
        let (view, playing) = frame("play");
        let (_, resting) = frame("rest");
        let size = |context: &Context, key: &str| {
            plan(Some(&view), context, &Stage::settled(Some(&view), context))
                .into_iter()
                .find(|placed| placed.key == key)
                .unwrap()
                .node
                .measure()
        };
        for key in ["tile", "pp"] {
            let (folded, open) = (size(&playing, key), size(&resting, key));
            assert!(
                folded.0 < open.0 * 0.7,
                "{key}: {folded:?} against {open:?}"
            );
        }
        assert!(size(&playing, "pp").1 < size(&resting, "pp").1 * 0.6);
        let mut stage = Stage::settled(Some(&view), &playing);
        let start = Instant::now();
        stage.advance(Some(&view), &resting, start);
        assert_eq!(stage.unfold, 0.0, "the first frame only starts the clock");
        stage.advance(
            Some(&view),
            &resting,
            start + Duration::from_secs_f32(UNFOLD_SECONDS / 2.0),
        );
        assert!(stage.unfold > 0.3 && stage.unfold < 0.7, "{}", stage.unfold);
        stage.advance(
            Some(&view),
            &resting,
            start + Duration::from_secs_f32(UNFOLD_SECONDS * 1.1),
        );
        assert_eq!(stage.unfold, 1.0);
    }

    #[test]
    fn the_rate_stands_over_the_meter_follows_its_scale_and_hides_with_it() {
        let (view, mut context) = frame("play");
        let rate = |context: &Context| {
            plan(Some(&view), context, &Stage::settled(Some(&view), context))
                .into_iter()
                .find(|placed| placed.key == "rate")
        };
        let usual = rate(&context).unwrap();
        assert_eq!(usual.anchor, Anchor::BottomCentre);
        assert!(
            (usual.x + usual.node.measure().0 / 2.0).abs() < 0.01,
            "it is centred on the meter"
        );
        context.play.as_mut().unwrap().meter = Some(Meter { scale: 1.5 });
        let large = rate(&context).unwrap();
        assert!(large.y > usual.y && large.node.measure().1 > usual.node.measure().1);
        context.play.as_mut().unwrap().meter = None;
        assert!(rate(&context).is_none(), "no meter in the game, no number");
        context.play.as_mut().unwrap().meter = Some(Meter { scale: 1.0 });
        context.play.as_mut().unwrap().unstable_rate = None;
        assert!(rate(&context).is_none());
    }

    #[test]
    fn the_shown_rate_moves_to_a_new_value_instead_of_jumping() {
        let (view, mut context) = frame("play");
        let mut stage = Stage::settled(Some(&view), &context);
        let start = Instant::now();
        stage.advance(Some(&view), &context, start);
        context.play.as_mut().unwrap().unstable_rate = Some(120.0);
        stage.advance(
            Some(&view),
            &context,
            start + Duration::from_secs_f32(RATE_HALF_LIFE),
        );
        let half = stage.rate.unwrap();
        assert!((half - 102.1).abs() < 0.6, "{half}");
        assert!(stage.moving(Some(&view), &context));
        for step in 1..=30 {
            stage.advance(
                Some(&view),
                &context,
                start + Duration::from_secs_f32(RATE_HALF_LIFE) + Duration::from_millis(step * 100),
            );
        }
        assert_eq!(stage.rate, Some(120.0));
    }

    #[test]
    fn every_plate_stays_inside_the_window_at_any_size() {
        for (wide, high) in [(1024, 768), (1280, 720), (1920, 1080), (2560, 1080)] {
            let viewport = Viewport {
                width: wide,
                height: high,
                scale: 1.0,
            };
            for (name, view, context) in sample::frames(Lang::En) {
                let stage = Stage::settled(Some(&view), &context);
                for sprite in sprites(Some(&view), &context, &stage, viewport) {
                    let lit = sprite
                        .pixels
                        .chunks_exact(4)
                        .enumerate()
                        .filter(|(_, pixel)| pixel[3] > 0);
                    for (at, _) in lit {
                        let x = sprite.x + (at as u32 % sprite.width) as i32;
                        let y = sprite.y + (at as u32 / sprite.width) as i32;
                        assert!(
                            x >= 0 && y >= 0 && x < wide as i32 && y < high as i32,
                            "{name} {wide}x{high}: {} leaves the window at {x}, {y}",
                            sprite.key
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn what_the_reader_tells_fills_the_play_and_a_play_without_pp_shows_no_counter() {
        let (mut view, _) = frame("play");
        let game = view.snapshot.gameplay.as_mut().unwrap();
        game.unstable_rate = Some(91.5);
        game.resting = Some(true);
        view.snapshot.meter = Some(dossier_overlay::Meter {
            shown: true,
            scale: 1.5,
        });
        let mut context = Context::default();
        told(&view, &mut context);
        let play = context.play.clone().unwrap();
        assert_eq!(
            (play.resting, play.unstable_rate, play.meter, play.pp),
            (true, Some(91.5), Some(Meter { scale: 1.5 }), None)
        );
        assert_eq!(
            keys(&view, &context),
            ["rate", "tile"],
            "the rate is shown, the counter waits for PP"
        );
        let game = view.snapshot.gameplay.as_mut().unwrap();
        game.pp = Some(212.4);
        game.pp_clean = Some(388.0);
        told(&view, &mut context);
        let play = context.play.clone().unwrap();
        assert_eq!((play.pp, play.clean), (Some(212.4), Some(388.0)));
        assert_eq!(keys(&view, &context), ["rate", "tile", "pp"]);
        view.snapshot.meter = Some(dossier_overlay::Meter {
            shown: false,
            scale: 1.5,
        });
        told(&view, &mut context);
        assert_eq!(
            context.play.unwrap().meter,
            None,
            "a meter that is off is no meter"
        );
    }

    #[test]
    fn a_context_travels_as_json_and_missing_parts_mean_nothing_to_show() {
        let (_, context) = frame("rest");
        let text = serde_json::to_string(&context).unwrap();
        assert_eq!(serde_json::from_str::<Context>(&text).unwrap(), context);
        let bare: Context = serde_json::from_str("{}").unwrap();
        assert_eq!(bare, Context::default());
        let (view, _) = frame("play");
        assert_eq!(keys(&view, &bare), ["tile"]);
    }
}
