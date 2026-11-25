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
pub const TICK_SECONDS: f32 = 0.45;
pub const RATE_HALF_LIFE: f32 = 0.12;
pub const METER_HIGH: f32 = 30.0;
pub const HOLD_SECONDS: f32 = 0.5;
pub const GLIDE_SECONDS: f32 = 0.22;
pub const SLIDE: f32 = 22.0;
pub const SCALES: [u16; 6] = [70, 80, 90, 100, 115, 130];

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
pub struct Profile {
    pub name: String,
    pub pp: f64,
    pub rank: u64,
    pub country: String,
    pub country_rank: u64,
    pub accuracy: f64,
    pub plays: u64,
    pub level: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub scale: u16,
    pub notices: bool,
    pub card: bool,
    pub pp: bool,
    pub rate: bool,
    pub maxpp: bool,
    pub maxcombo: bool,
    pub profile: bool,
    pub duel: bool,
}

impl Default for Look {
    fn default() -> Look {
        Look {
            scale: 100,
            notices: true,
            card: true,
            pp: true,
            rate: true,
            maxpp: true,
            maxcombo: true,
            profile: true,
            duel: true,
        }
    }
}

impl Look {
    pub fn share(&self) -> f32 {
        f32::from(self.scale.clamp(SCALES[0], SCALES[SCALES.len() - 1])) / 100.0
    }
}

pub fn nearest_scale(scale: u16) -> usize {
    SCALES
        .iter()
        .enumerate()
        .min_by_key(|(_, step)| step.abs_diff(scale))
        .map_or(0, |(at, _)| at)
}

pub fn scale_step(scale: u16, by: i32) -> u16 {
    let at = nearest_scale(scale) as i32 + by;
    SCALES[at.clamp(0, SCALES.len() as i32 - 1) as usize]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Panel {
    Tuning,
    Hidden,
    Board,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Context {
    pub lang: Lang,
    pub card: Option<Card>,
    pub play: Option<Play>,
    pub outcome: Option<Outcome>,
    pub notices: Vec<Notice>,
    pub look: Look,
    pub hidden: bool,
    pub panel: Option<Panel>,
    pub board: f32,
    pub profile: Option<Profile>,
    pub profile_shown: f32,
    pub duel: Option<u64>,
    pub greet: bool,
    pub ticked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stage {
    pub unfold: f32,
    pub rate: Option<f32>,
    pub pp: Option<f32>,
    pub greet: f32,
    pub tick: f32,
    last: Option<Instant>,
}

fn chased(shown: Option<f32>, wanted: Option<f32>, seconds: f32, near: f32) -> Option<f32> {
    match (shown, wanted) {
        (Some(shown), Some(aim)) => {
            let keep = 0.5f32.powf(seconds / RATE_HALF_LIFE);
            let next = aim + (shown - aim) * keep;
            Some(if (next - aim).abs() < near { aim } else { next })
        }
        (_, aim) => aim,
    }
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
    if game.pp_record.is_some() {
        play.record = game.pp_record;
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
            pp: context.play.as_ref().and_then(|play| play.pp).map(|pp| pp as f32),
            greet: if context.greet { 1.0 } else { 0.0 },
            tick: if context.ticked { 1.0 } else { 0.0 },
            last: None,
        }
    }

    pub fn advance(&mut self, view: Option<&View>, context: &Context, now: Instant) {
        let dt = self
            .last
            .map_or(0.0, |was| now.saturating_duration_since(was).as_secs_f32())
            .min(0.25);
        self.last = Some(now);
        let resting = context.play.as_ref().is_none_or(|play| play.resting);
        let aim = if live(view) && !resting { 0.0 } else { 1.0 };
        let step = dt / UNFOLD_SECONDS;
        self.unfold = toward(self.unfold, aim, step);
        self.greet = toward(self.greet, if context.greet { 1.0 } else { 0.0 }, step);
        self.tick = if context.ticked {
            (self.tick + dt / TICK_SECONDS).min(1.0)
        } else {
            0.0
        };
        let wanted = context
            .play
            .as_ref()
            .filter(|_| live(view))
            .and_then(|play| play.unstable_rate)
            .map(|rate| rate as f32);
        self.rate = chased(self.rate, wanted, dt, 0.05);
        let reached = context.play.as_ref().filter(|_| live(view)).and_then(|play| play.pp).map(|pp| pp as f32);
        self.pp = chased(self.pp, reached, dt, 0.3);
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
            || (self.greet - if context.greet { 1.0 } else { 0.0 }).abs() > 0.001
            || (context.ticked && self.tick < 1.0)
            || live(view)
            || matches!((self.rate, wanted), (Some(shown), Some(aim)) if (shown - aim).abs() > 0.05)
    }

    pub fn eased(&self) -> f32 {
        smooth(self.unfold)
    }

    pub fn greeted(&self) -> f32 {
        smooth(self.greet)
    }
}

fn smooth(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

fn toward(from: f32, aim: f32, step: f32) -> f32 {
    if from < aim {
        (from + step).min(aim)
    } else {
        (from - step).max(aim)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    BottomCentre,
    TopCentre,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub glide: f32,
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
            glide: 1.0,
                key,
                anchor,
                x: EDGE,
                y: pen,
                node,
            };
            pen += high + GAP;
            placed
        })
        .collect()
}

const GAP: f32 = 8.0;

fn top_centre(key: &'static str, node: Node) -> Placed {
    let wide = node.measure().0;
    Placed {
            glide: 1.0,
        key,
        anchor: Anchor::TopCentre,
        x: -wide / 2.0,
        y: EDGE,
        node,
    }
}

pub fn plan(view: Option<&View>, context: &Context, stage: &Stage) -> Vec<Placed> {
    let Some(view) = view else {
        return Vec::new();
    };
    let lang = context.lang;
    let look = context.look;
    if context.hidden {
        return match context.panel {
            Some(Panel::Hidden) => vec![top_centre("hidden", plates::hidden(lang))],
            _ => Vec::new(),
        };
    }
    let screen = view.snapshot.screen;
    let playing = live(Some(view));
    let mut placed = Vec::new();
    if context.panel == Some(Panel::Tuning) && !playing {
        placed.push(top_centre("tuning", plates::tuning(look.scale, lang)));
    }
    let board = context.board > 0.01;
    if board {
        let node = plates::board(&context.notices, lang);
        placed.push(Placed {
            glide: context.board,
            key: "board",
            anchor: Anchor::TopRight,
            x: EDGE,
            y: EDGE,
            node,
        });
    }
    let tick = context.ticked.then_some(stage.tick);
    let mut corner = vec![("tile", plates::tile(lang, stage.greeted(), tick))];
    match screen {
        Screen::Menu => {
            let notices: Vec<Node> = context
                .notices
                .iter()
                .filter(|_| look.notices && !board)
                .take(2)
                .map(plates::notice)
                .collect();
            placed.extend(stacked(
                ["notice-1", "notice-2"],
                Anchor::TopRight,
                96.0,
                notices,
            ));
        }
        Screen::Selection => {
            // Where the ceiling stands does not depend on the chat's top, which comes later than the map:
            // a plate that has been dragged is placed from this spot, and it must not move when the top arrives.
            let above = EDGE + plates::card_height() + GAP;
            if let Some(node) = context
                .card
                .as_ref()
                .filter(|_| look.card)
                .and_then(|card| plates::card(card, lang))
            {
                placed.push(Placed {
                    glide: 1.0,
                    key: "card",
                    anchor: Anchor::BottomLeft,
                    x: EDGE,
                    y: EDGE,
                    node,
                });
            }
            if let Some(pp) = view
                .snapshot
                .beatmap
                .as_ref()
                .and_then(|map| map.max_pp)
                .filter(|pp| pp.is_finite() && *pp > 0.0 && look.maxpp)
            {
                let mods = view.snapshot.beatmap.as_ref().map_or(0, |map| map.max_pp_mods);
                placed.push(Placed {
                    glide: 1.0,
                    key: "maxpp",
                    anchor: Anchor::BottomLeft,
                    x: EDGE,
                    y: above,
                    node: plates::max_pp(pp, mods, lang),
                });
            }
            if let Some(combo) = view
                .snapshot
                .beatmap
                .as_ref()
                .and_then(|map| map.max_combo)
                .filter(|combo| *combo > 0 && look.maxcombo)
            {
                // Beside the ceiling, at a place of its own that depends on nothing else on the screen.
                placed.push(Placed {
                    glide: 1.0,
                    key: "maxcombo",
                    anchor: Anchor::BottomLeft,
                    x: EDGE + plates::MAX_PP_WIDE + GAP,
                    y: above,
                    node: plates::max_combo(combo, lang),
                });
            }
        }
        Screen::Playing if playing => {
            if let Some(play) = &context.play {
                if let Some(pp) = play.pp.filter(|_| look.pp) {
                    let shown = stage.pp.map_or(pp, f64::from);
                    corner.push(("pp", plates::reach(play, shown, lang, stage.eased())));
                }
                if let (Some(meter), Some(rate)) = (play.meter.filter(|_| look.rate), stage.rate) {
                    let node = plates::rate(rate, lang, meter.scale);
                    let wide = node.measure().0;
                    placed.push(Placed {
            glide: 1.0,
                        key: "rate",
                        anchor: Anchor::BottomCentre,
                        x: -wide / 2.0,
                        y: METER_HIGH / (SIZE * look.share()) * meter.scale.clamp(0.5, 3.0) + 6.0,
                        node,
                    });
                }
            }
        }
        Screen::Results => {
            if let Some(outcome) = &context.outcome {
                if let Some(key) = outcome.offer.as_deref().filter(|_| outcome.saved) {
                    corner.push(("offer", plates::offer(key, outcome.holding, lang)));
                }
            }
        }
        _ => {}
    }
    if let Some(profile) = context.profile.as_ref().filter(|_| look.profile) {
        let glide = context.profile_shown.max(context.board);
        if glide > 0.01 {
            placed.push(Placed {
                glide,
                key: "profile",
                anchor: Anchor::TopLeft,
                x: EDGE,
                y: EDGE,
                node: plates::profile(profile, lang),
            });
        }
    }
    if let Some(seconds) = context.duel.filter(|_| look.duel && !playing) {
        let tuning = context.panel == Some(Panel::Tuning);
        let node = plates::duel(seconds, lang);
        let wide = node.measure().0;
        let drop = if tuning { 130.0 } else { 0.0 };
        placed.push(Placed {
            glide: 1.0,
            key: "duel",
            anchor: Anchor::TopCentre,
            x: -wide / 2.0,
            y: EDGE + drop,
            node,
        });
    }
    let mut pen = EDGE;
    for (key, node) in corner {
        let high = node.measure().1;
        placed.push(Placed {
            glide: 1.0,
            key,
            anchor: Anchor::BottomRight,
            x: EDGE,
            y: pen,
            node,
        });
        pen += high + GAP;
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
    /// Where the cross that closes the plate is, as its centre and radius in the sprite's pixels.
    pub cross: Option<(i32, i32, i32)>,
}

/// The plates that have a cross on them while the panel is open, as they are named by their keys.
pub fn closable(key: &str) -> bool {
    matches!(key, "pp" | "rate" | "notice-1" | "notice-2" | "card" | "maxpp" | "maxcombo" | "profile" | "duel" | "board")
}

/// The switch in `Look` that a plate answers to, or none for the plates that have no switch.
pub fn switch_of(key: &str) -> Option<&'static str> {
    match key {
        "pp" => Some("pp"),
        "rate" => Some("rate"),
        "notice-1" | "notice-2" => Some("notices"),
        "card" => Some("card"),
        "maxpp" => Some("maxpp"),
        "maxcombo" => Some("maxcombo"),
        "profile" => Some("profile"),
        "duel" => Some("duel"),
        _ => None,
    }
}

pub fn unit(viewport: Viewport, look: Look) -> f32 {
    (viewport.height as f32 / REFERENCE_HIGH * SIZE * look.share()).clamp(0.4, 5.0)
}

pub fn paint(placed: &Placed, viewport: Viewport, unit: f32, crossed: bool) -> Option<Sprite> {
    let (wide, high) = placed.node.measure();
    let room = 12.0;
    let mut canvas = draw::Canvas::new(wide + room * 2.0, high + room * 2.0, unit)?;
    canvas.set_alpha(placed.glide);
    placed.node.draw(&mut canvas, room, room, wide);
    let cross = (crossed && closable(placed.key)).then(|| {
        let (cx, cy, radius) = (room + wide - 3.0, room + 3.0, 8.0);
        canvas.disc(cx, cy, radius, draw::rgb(0x3a1216, 0.96));
        canvas.line(&[(cx - 3.0, cy - 3.0), (cx + 3.0, cy + 3.0)], 1.6, draw::rgb(0xf4e9e6, 1.0));
        canvas.line(&[(cx - 3.0, cy + 3.0), (cx + 3.0, cy - 3.0)], 1.6, draw::rgb(0xf4e9e6, 1.0));
        (
            (cx * unit).round() as i32,
            (cy * unit).round() as i32,
            (radius * unit).round().max(6.0) as i32,
        )
    });
    let (frame_wide, frame_high) = (viewport.width as f32 / unit, viewport.height as f32 / unit);
    let (left, top) = match placed.anchor {
        Anchor::TopLeft => (placed.x, placed.y),
        Anchor::TopRight => (frame_wide - placed.x - wide, placed.y),
        Anchor::BottomLeft => (placed.x, frame_high - placed.y - high),
        Anchor::BottomRight => (frame_wide - placed.x - wide, frame_high - placed.y - high),
        Anchor::BottomCentre => (frame_wide / 2.0 + placed.x, frame_high - placed.y - high),
        Anchor::TopCentre => (frame_wide / 2.0 + placed.x, placed.y),
    };
    let slide = (1.0 - placed.glide.clamp(0.0, 1.0)) * SLIDE;
    let (left, top) = match placed.anchor {
        Anchor::TopRight | Anchor::BottomRight => (left + slide, top),
        Anchor::TopLeft | Anchor::BottomLeft => (left - slide, top),
        Anchor::TopCentre => (left, top - slide),
        Anchor::BottomCentre => (left, top + slide),
    };
    Some(Sprite {
        key: placed.key,
        x: ((left - room) * unit).round() as i32,
        y: ((top - room) * unit).round() as i32,
        width: canvas.pixmap.width(),
        height: canvas.pixmap.height(),
        pixels: canvas.pixmap.take(),
        cross,
    })
}

pub fn sprites(
    view: Option<&View>,
    context: &Context,
    stage: &Stage,
    viewport: Viewport,
) -> Vec<Sprite> {
    let unit = unit(viewport, context.look);
    plan(view, context, stage)
        .iter()
        .filter_map(|placed| paint(placed, viewport, unit, context.board > 0.01))
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
        assert_eq!(keys(&view, &context), ["notice-1", "notice-2", "tile"]);
        let (view, context) = frame("select");
        assert_eq!(keys(&view, &context), ["card", "tile"]);
        let (view, context) = frame("play");
        assert_eq!(keys(&view, &context), ["rate", "tile", "pp"]);
        for name in ["results", "hold", "saved"] {
            let (view, context) = frame(name);
            assert_eq!(keys(&view, &context), ["tile", "offer"], "{name}");
        }
        let (view, mut context) = frame("select");
        context.card.as_mut().unwrap().places.clear();
        assert_eq!(keys(&view, &context), ["tile"], "a pool alone is no leaderboard");
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
        let (folded, open) = (size(&playing, "pp"), size(&resting, "pp"));
        assert!(folded.0 < open.0 * 0.7, "{folded:?} against {open:?}");
        assert_eq!(size(&playing, "tile"), size(&resting, "tile"), "the tile is only the logo in a play");
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
        for ((wide, high), scale) in [(1024, 768), (1280, 720), (1920, 1080), (2560, 1080)]
            .into_iter()
            .flat_map(|size| [SCALES[0], 100, SCALES[SCALES.len() - 1]].map(|scale| (size, scale)))
        {
            let viewport = Viewport {
                width: wide,
                height: high,
                scale: 1.0,
            };
            for (name, view, mut context) in sample::frames(Lang::En) {
                context.look.scale = scale;
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
                            "{name} {wide}x{high} at {scale} %: {} leaves the window at {x}, {y}",
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
    fn the_pp_counter_runs_to_a_new_value_instead_of_jumping() {
        let (view, mut context) = frame("play");
        context.play.as_mut().unwrap().pp = Some(100.0);
        let mut stage = Stage::settled(Some(&view), &context);
        assert_eq!(stage.pp, Some(100.0));
        let from = Instant::now();
        stage.advance(Some(&view), &context, from);
        context.play.as_mut().unwrap().pp = Some(140.0);
        stage.advance(Some(&view), &context, from + Duration::from_millis(60));
        let midway = stage.pp.unwrap();
        assert!(midway > 105.0 && midway < 135.0, "{midway}");
        for step in 2..40 {
            stage.advance(Some(&view), &context, from + Duration::from_millis(60) * step);
        }
        assert_eq!(stage.pp, Some(140.0));
        context.play.as_mut().unwrap().pp = None;
        stage.advance(Some(&view), &context, from + Duration::from_secs(4));
        assert_eq!(stage.pp, None);
    }

    #[test]
    fn the_look_turns_plates_off_and_sizes_all_of_them() {
        let (view, mut context) = frame("menu");
        context.look.notices = false;
        assert_eq!(keys(&view, &context), ["tile"]);
        let (view, mut context) = frame("play");
        context.look.pp = false;
        assert_eq!(keys(&view, &context), ["rate", "tile"]);
        context.look.rate = false;
        assert_eq!(keys(&view, &context), ["tile"]);
        let (view, mut context) = frame("select");
        context.look.card = false;
        assert_eq!(keys(&view, &context), ["tile"]);
        let viewport = Viewport {
            width: 1280,
            height: 720,
            scale: 1.0,
        };
        let (view, mut context) = frame("menu");
        let wide = |context: &Context| {
            sprites(Some(&view), context, &Stage::settled(Some(&view), context), viewport)
                .into_iter()
                .find(|sprite| sprite.key == "notice-1")
                .unwrap()
                .width
        };
        let usual = wide(&context);
        context.look.scale = 130;
        assert!(wide(&context) as f32 > usual as f32 * 1.25);
        context.look.scale = 70;
        assert!((wide(&context) as f32) < usual as f32 * 0.75);
    }

    #[test]
    fn the_tuning_panel_stays_out_of_a_play_and_hidden_plates_leave_only_their_note() {
        let (view, mut context) = frame("menu");
        context.panel = Some(Panel::Tuning);
        assert_eq!(keys(&view, &context), ["tuning", "notice-1", "notice-2", "tile"]);
        let (view, mut playing) = frame("play");
        playing.panel = Some(Panel::Tuning);
        assert_eq!(keys(&view, &playing), ["rate", "tile", "pp"]);
        let (view, mut context) = frame("menu");
        context.hidden = true;
        context.panel = Some(Panel::Hidden);
        assert_eq!(keys(&view, &context), ["hidden"]);
        context.panel = None;
        assert!(keys(&view, &context).is_empty());
    }

    #[test]
    fn the_tile_greets_once_with_words_and_a_saved_play_draws_a_tick_in_place_of_the_logo() {
        let (view, greeting) = frame("menu");
        let tile = |context: &Context, stage: &Stage| {
            plan(Some(&view), context, stage)
                .into_iter()
                .find(|placed| placed.key == "tile")
                .unwrap()
                .node
        };
        let quiet = Context {
            greet: false,
            ..greeting.clone()
        };
        let wide = |context: &Context| tile(context, &Stage::settled(Some(&view), context)).measure().0;
        assert!(wide(&greeting) > wide(&quiet) * 3.0, "the words fold away to the logo");
        let mut stage = Stage::settled(Some(&view), &greeting);
        let start = Instant::now();
        stage.advance(Some(&view), &quiet, start);
        stage.advance(Some(&view), &quiet, start + Duration::from_secs_f32(UNFOLD_SECONDS / 2.0));
        assert!(stage.greet > 0.3 && stage.greet < 0.7, "{}", stage.greet);
        let ticked = Context {
            ticked: true,
            ..quiet.clone()
        };
        let mut stage = Stage::settled(Some(&view), &quiet);
        stage.advance(Some(&view), &ticked, start);
        stage.advance(Some(&view), &ticked, start + Duration::from_secs_f32(TICK_SECONDS / 2.0));
        assert!(stage.tick > 0.3 && stage.tick < 0.7, "the tick draws itself: {}", stage.tick);
        assert!(stage.moving(Some(&view), &ticked));
        assert!(matches!(tile(&ticked, &stage), Node::Morph { from, .. } if matches!(*from, Node::Tick { .. })));
        assert!(matches!(tile(&quiet, &stage), Node::Morph { from, .. } if matches!(*from, Node::Logo { .. })));
    }

    #[test]
    fn the_scale_moves_along_its_steps_and_stops_at_the_ends() {
        assert_eq!(scale_step(100, 1), 115);
        assert_eq!(scale_step(100, -1), 90);
        assert_eq!(scale_step(130, 1), 130);
        assert_eq!(scale_step(70, -1), 70);
        assert_eq!(scale_step(104, 1), 115, "a value between steps starts from the nearest");
        assert_eq!(Look { scale: 400, ..Look::default() }.share(), 1.3);
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

    #[test]
    fn song_select_shows_the_ceiling_of_the_map_and_other_screens_do_not() {
        let (mut view, context) = frame("select");
        assert!(!keys(&view, &context).contains(&"maxpp"), "a map without a ceiling shows none");
        view.snapshot.beatmap = Some(dossier_overlay::Beatmap {
            md5: "a".repeat(32),
            title: String::new(),
            artist: String::new(),
            difficulty: String::new(),
            max_pp: Some(412.6),
            max_pp_mods: 0,
            max_combo: None,
        });
        assert!(keys(&view, &context).contains(&"maxpp"));
        let off = Context { look: Look { maxpp: false, ..Look::default() }, ..context.clone() };
        assert!(!keys(&view, &off).contains(&"maxpp"), "its own switch turns it off");
        let (mut menu, menu_context) = frame("menu");
        menu.snapshot.beatmap = view.snapshot.beatmap.clone();
        assert!(!keys(&menu, &menu_context).contains(&"maxpp"));
    }

    #[test]
    fn the_profile_is_one_plate_and_fades_with_its_glide() {
        let (view, context) = frame("menu");
        let shown = Context {
            profile: Some(Profile { name: "hmosha".into(), rank: 6546, ..Profile::default() }),
            profile_shown: 0.5,
            ..context.clone()
        };
        let plates = plan(Some(&view), &shown, &Stage::settled(Some(&view), &shown));
        let mine: Vec<_> = plates.iter().filter(|p| p.key == "profile").collect();
        assert_eq!(mine.len(), 1);
        assert!((mine[0].glide - 0.5).abs() < 1e-6);
        let closed = Context { profile_shown: 0.0, ..shown };
        assert!(!keys(&view, &closed).contains(&"profile"));
    }

    #[test]
    fn the_profile_opens_with_the_board_as_well_as_on_its_own_key() {
        let (view, context) = frame("menu");
        let base = Context { profile: Some(Profile { name: "a".into(), ..Profile::default() }), ..context };
        assert!(!keys(&view, &base).contains(&"profile"), "neither is open");
        let board = Context { board: 1.0, ..base.clone() };
        assert!(keys(&view, &board).contains(&"profile"));
        let own = Context { profile_shown: 1.0, ..base };
        assert!(keys(&view, &own).contains(&"profile"));
    }

    #[test]
    fn the_ceiling_stands_still_when_the_chats_top_arrives() {
        let (mut view, context) = frame("select");
        view.snapshot.beatmap = Some(dossier_overlay::Beatmap {
            md5: "a".repeat(32),
            title: String::new(),
            artist: String::new(),
            difficulty: String::new(),
            max_pp: Some(300.0),
            max_pp_mods: 0,
            max_combo: None,
        });
        let at = |context: &Context| {
            plan(Some(&view), context, &Stage::settled(Some(&view), context))
                .into_iter()
                .find(|placed| placed.key == "maxpp")
                .map(|placed| (placed.x, placed.y))
        };
        let without = Context { card: None, ..context.clone() };
        assert!(context.card.is_some(), "the sample has a top");
        assert_eq!(at(&without), at(&context));
    }

    #[test]
    fn crosses_are_drawn_only_while_the_panel_is_open_and_only_on_plates_that_can_be_closed() {
        let (view, context) = frame("menu");
        let sprites = |context: &Context| {
            let viewport = Viewport { width: 1280, height: 720, scale: 1.0 };
            sprites(Some(&view), context, &Stage::settled(Some(&view), context), viewport)
        };
        assert!(sprites(&context).iter().all(|sprite| sprite.cross.is_none()), "no panel, no crosses");
        let open = Context { board: 1.0, ..context.clone() };
        let drawn = sprites(&open);
        for sprite in &drawn {
            assert_eq!(sprite.cross.is_some(), closable(sprite.key), "{}", sprite.key);
        }
        assert!(drawn.iter().any(|sprite| sprite.key == "board" && sprite.cross.is_some()));
        assert!(drawn.iter().any(|sprite| sprite.key == "tile" && sprite.cross.is_none()), "the tile has no switch");
        let (x, y, radius) = drawn.iter().find(|sprite| sprite.key == "board").and_then(|sprite| sprite.cross).unwrap();
        assert!(x > 0 && y > 0 && radius >= 6);
    }

    #[test]
    fn a_duel_search_shows_its_time_unless_switched_off_or_in_a_play() {
        let (view, context) = frame("menu");
        let searching = Context { duel: Some(65), ..context };
        assert!(keys(&view, &searching).contains(&"duel"));
        let off = Context { look: Look { duel: false, ..Look::default() }, ..searching.clone() };
        assert!(!keys(&view, &off).contains(&"duel"));
        let (play, _) = frame("play");
        assert!(!keys(&play, &searching).contains(&"duel"), "a play is not interrupted by the queue");
    }

    #[test]
    fn every_closable_plate_but_the_panel_has_a_switch() {
        for key in ["pp", "rate", "notice-1", "notice-2", "card", "maxpp", "maxcombo", "profile", "duel"] {
            assert!(closable(key) && switch_of(key).is_some(), "{key}");
        }
        assert!(closable("board") && switch_of("board").is_none());
        assert!(!closable("tile") && switch_of("tile").is_none());
    }

    #[test]
    fn the_max_combo_stands_beside_the_ceiling_and_has_its_own_switch() {
        let (mut view, context) = frame("select");
        view.snapshot.beatmap = Some(dossier_overlay::Beatmap {
            md5: "a".repeat(32),
            title: String::new(),
            artist: String::new(),
            difficulty: String::new(),
            max_pp: Some(300.0),
            max_pp_mods: 0,
            max_combo: Some(1843),
        });
        let find = |context: &Context, key: &str| {
            plan(Some(&view), context, &Stage::settled(Some(&view), context)).into_iter().find(|placed| placed.key == key)
        };
        let (pp, combo) = (find(&context, "maxpp").unwrap(), find(&context, "maxcombo").unwrap());
        assert_eq!(pp.y, combo.y, "they stand on one line");
        assert!(combo.x >= pp.x + pp.node.measure().0, "and do not overlap");
        let without_pp = Context { look: Look { maxpp: false, ..Look::default() }, ..context.clone() };
        assert_eq!(find(&without_pp, "maxcombo").map(|placed| placed.x), Some(combo.x), "it does not move when the ceiling is switched off");
        let off = Context { look: Look { maxcombo: false, ..Look::default() }, ..context.clone() };
        assert!(find(&off, "maxcombo").is_none());
        let mut unknown = view.clone();
        unknown.snapshot.beatmap.as_mut().unwrap().max_combo = None;
        let plates = plan(Some(&unknown), &context, &Stage::settled(Some(&unknown), &context));
        assert!(plates.iter().all(|placed| placed.key != "maxcombo"), "a map without a known combo shows none");
    }
}
