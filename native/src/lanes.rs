use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{overlay, renderer, Clipboard, Shell};
use iced::{mouse, Element, Length, Point, Rectangle, Size, Theme, Transformation, Vector};

use crate::board::{Spot, FLOW};

type Renderer = iced::Renderer;

const LINE: f32 = 84.0;
const RATE: f32 = 13.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Lane<K> {
    pub x: f32,
    pub width: f32,
    pub keys: Vec<K>,
    pub fill: bool,
}

impl<K> Lane<K> {
    pub fn stack(x: f32, width: f32, keys: impl Into<Vec<K>>) -> Self {
        Lane { x, width, keys: keys.into(), fill: false }
    }

    pub fn fill(x: f32, width: f32, key: K) -> Self {
        Lane { x, width, keys: vec![key], fill: true }
    }
}

pub fn width_of<K: PartialEq>(lanes: &[Lane<K>], key: K) -> Option<f32> {
    lanes.iter().find(|lane| lane.keys.contains(&key)).map(|lane| lane.width)
}

pub struct Lanes<'a, Message, K> {
    pieces: Vec<(K, Element<'a, Message>)>,
    plan: Box<dyn Fn(f32) -> Vec<Lane<K>> + 'a>,
    spacing: f32,
    top: f32,
    bottom: f32,
}

pub fn lanes<'a, Message: 'a, K: Copy + Eq + Hash + 'static>(pieces: Vec<(K, Element<'a, Message>)>, plan: impl Fn(f32) -> Vec<Lane<K>> + 'a) -> Lanes<'a, Message, K> {
    Lanes { pieces, plan: Box::new(plan), spacing: 12.0, top: 0.0, bottom: 0.0 }
}

impl<'a, Message, K> Lanes<'a, Message, K> {
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn room(mut self, top: f32, bottom: f32) -> Self {
        self.top = top;
        self.bottom = bottom;
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Roll {
    at: f32,
    pending: f32,
    most: f32,
}

struct State<K> {
    keys: Vec<K>,
    shape: Vec<Vec<K>>,
    lanes: Vec<Lane<K>>,
    rolls: Vec<Roll>,
    targets: HashMap<K, (usize, Point)>,
    spots: HashMap<K, Spot>,
    last: Option<Instant>,
}

impl<K: Copy + Eq + Hash> State<K> {
    fn new(keys: Vec<K>) -> Self {
        State { keys, shape: Vec::new(), lanes: Vec::new(), rolls: Vec::new(), targets: HashMap::new(), spots: HashMap::new(), last: None }
    }

    fn rolled(&self, key: &K) -> f32 {
        self.targets.get(key).and_then(|(lane, _)| self.rolls.get(*lane)).map_or(0.0, |roll| roll.at)
    }

    fn shown(&self, key: &K) -> Option<Point> {
        let spot = self.spots.get(key)?.point();
        Some(Point::new(spot.x, spot.y - self.rolled(key)))
    }

    fn replan(&mut self, lanes: Vec<Lane<K>>, targets: HashMap<K, (usize, Point)>, mosts: Vec<f32>) {
        let shape: Vec<Vec<K>> = lanes.iter().map(|lane| lane.keys.clone()).collect();
        if shape != self.shape {
            for (key, spot) in self.spots.iter_mut() {
                let rolled = self.targets.get(key).and_then(|(lane, _)| self.rolls.get(*lane)).map_or(0.0, |roll| roll.at);
                spot.moved_by(Vector::new(0.0, -rolled));
            }
            self.rolls = vec![Roll::default(); lanes.len()];
            self.shape = shape;
        }
        for (roll, most) in self.rolls.iter_mut().zip(mosts) {
            roll.most = most;
            roll.at = roll.at.clamp(0.0, most);
        }
        self.spots.retain(|key, _| targets.contains_key(key));
        for (key, (_, target)) in &targets {
            self.spots.entry(*key).or_insert_with(|| Spot::at(*target));
        }
        self.targets = targets;
        self.lanes = lanes;
    }

    fn step(&mut self, now: Instant) -> bool {
        let dt = match self.last {
            Some(last) if now < last => return true,
            Some(last) => now.duration_since(last).as_secs_f32().min(0.05),
            None => 1.0 / 60.0,
        };
        let mut moving = false;
        for (key, spot) in self.spots.iter_mut() {
            if let Some((_, target)) = self.targets.get(key) {
                moving |= spot.step(*target, dt, FLOW);
            }
        }
        for roll in self.rolls.iter_mut() {
            if roll.pending.abs() > 0.25 {
                let step = roll.pending * (1.0 - (-RATE * dt).exp());
                roll.pending -= step;
                let before = roll.at;
                roll.at = (roll.at + step).clamp(0.0, roll.most);
                if roll.at == before {
                    roll.pending = 0.0;
                }
                moving = true;
            } else {
                roll.pending = 0.0;
            }
        }
        self.last = if moving { Some(now) } else { None };
        moving
    }

    fn lane_under(&self, x: f32) -> Option<usize> {
        self.lanes.iter().position(|lane| !lane.fill && x >= lane.x && x <= lane.x + lane.width)
    }
}

fn shifted(cursor: mouse::Cursor, by: Vector) -> mouse::Cursor {
    match cursor {
        mouse::Cursor::Available(p) => mouse::Cursor::Available(p - by),
        other => other,
    }
}

impl<Message, K: Copy + Eq + Hash + 'static> Lanes<'_, Message, K> {
    fn keys(&self) -> Vec<K> {
        self.pieces.iter().map(|(key, _)| *key).collect()
    }
}

impl<Message, K: Copy + Eq + Hash + 'static> Widget<Message, Theme, Renderer> for Lanes<'_, Message, K> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<K>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.keys()))
    }

    fn children(&self) -> Vec<Tree> {
        self.pieces.iter().map(|(_, piece)| Tree::new(piece)).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let keys = self.keys();
        let state = tree.state.downcast_mut::<State<K>>();
        let before = std::mem::replace(&mut state.keys, keys);
        let mut old: HashMap<K, Tree> = before.into_iter().zip(tree.children.drain(..)).collect();
        tree.children = self
            .pieces
            .iter()
            .map(|(key, piece)| match old.remove(key) {
                Some(mut kept) => {
                    kept.diff(piece);
                    kept
                }
                None => Tree::new(piece),
            })
            .collect();
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let size = limits.max();
        let lanes: Vec<Lane<K>> = (self.plan)(size.width)
            .into_iter()
            .map(|mut lane| {
                lane.keys.retain(|key| self.pieces.iter().any(|(own, _)| own == key));
                lane
            })
            .collect();
        let mut nodes: Vec<Node> = self.pieces.iter().map(|_| Node::new(Size::ZERO)).collect();
        let mut targets = HashMap::new();
        let mut mosts = Vec::with_capacity(lanes.len());
        for (at, lane) in lanes.iter().enumerate() {
            let mut y = if lane.fill { 0.0 } else { self.top };
            for key in &lane.keys {
                let Some(index) = self.pieces.iter().position(|(own, _)| own == key) else {
                    continue;
                };
                let fits = if lane.fill {
                    layout::Limits::new(Size::new(lane.width, size.height), Size::new(lane.width, size.height))
                } else {
                    layout::Limits::new(Size::new(lane.width, 0.0), Size::new(lane.width, f32::INFINITY))
                };
                let node = self.pieces[index].1.as_widget_mut().layout(&mut tree.children[index], renderer, &fits);
                let height = node.size().height;
                let target = Point::new(lane.x, y);
                nodes[index] = node.move_to(target);
                targets.insert(*key, (at, target));
                y += height + self.spacing;
                if lane.fill {
                    break;
                }
            }
            let content = if lane.keys.is_empty() || lane.fill { 0.0 } else { y - self.spacing + self.bottom };
            mosts.push((content - size.height).max(0.0));
        }
        tree.state.downcast_mut::<State<K>>().replan(lanes, targets, mosts);
        Node::with_children(size, nodes)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        let state = tree.state.downcast_ref::<State<K>>();
        let shown: Vec<bool> = self.pieces.iter().map(|(key, _)| state.targets.contains_key(key)).collect();
        for ((((_, piece), child), place), shown) in self.pieces.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()).zip(shown) {
            if shown {
                piece.as_widget_mut().operate(child, place, renderer, operation);
            }
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);
        let state = tree.state.downcast_mut::<State<K>>();
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if state.step(*now) {
                shell.request_redraw();
            }
        }
        let inside = if cursor.is_over(bounds) { cursor } else { mouse::Cursor::Unavailable };
        let offsets: Vec<Option<Vector>> = self
            .pieces
            .iter()
            .zip(layout.children())
            .map(|((key, _), place)| state.shown(key).map(|at| at + origin - place.bounds().position()))
            .collect();
        for ((((_, piece), child), place), by) in self.pieces.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()).zip(offsets) {
            let Some(by) = by else {
                continue;
            };
            piece.as_widget_mut().update(child, event, place, shifted(inside, by), renderer, clipboard, shell, viewport);
        }
        if shell.is_event_captured() {
            return;
        }
        let iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event else {
            return;
        };
        let Some(at) = cursor.position_in(bounds) else {
            return;
        };
        let state = tree.state.downcast_mut::<State<K>>();
        let Some(lane) = state.lane_under(at.x) else {
            return;
        };
        let roll = &mut state.rolls[lane];
        if roll.most <= 0.0 {
            return;
        }
        match *delta {
            mouse::ScrollDelta::Lines { y, .. } => roll.pending -= y * LINE,
            mouse::ScrollDelta::Pixels { y, .. } => {
                roll.pending = 0.0;
                roll.at = (roll.at - y).clamp(0.0, roll.most);
            }
        }
        state.last = None;
        shell.capture_event();
        shell.request_redraw();
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let bounds = layout.bounds();
        if !cursor.is_over(bounds) {
            return mouse::Interaction::None;
        }
        let state = tree.state.downcast_ref::<State<K>>();
        let origin = Vector::new(bounds.x, bounds.y);
        self.pieces
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .filter_map(|(((key, piece), child), place)| {
                let by = state.shown(key)? + origin - place.bounds().position();
                Some(piece.as_widget().mouse_interaction(child, place, shifted(cursor, by), viewport, renderer))
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let state = tree.state.downcast_ref::<State<K>>();
        let origin = Vector::new(bounds.x, bounds.y);
        let inside = if cursor.is_over(bounds) { cursor } else { mouse::Cursor::Unavailable };
        let mut order: Vec<(u8, usize)> = self
            .pieces
            .iter()
            .enumerate()
            .filter_map(|(index, (key, _))| {
                let (lane, target) = state.targets.get(key)?;
                let shown = state.shown(key)?;
                let resting = (shown.x - target.x).abs() < 0.5 && (shown.y + state.rolled(key) - target.y).abs() < 0.5;
                let fill = state.lanes.get(*lane).is_some_and(|lane| lane.fill);
                Some((if fill { 0 } else if resting { 1 } else { 2 }, index))
            })
            .collect();
        order.sort();
        let children: Vec<Layout<'_>> = layout.children().collect();
        renderer.with_layer(clip, |renderer| {
            for (_, index) in order {
                let (key, piece) = &self.pieces[index];
                let place = children[index];
                let Some(shown) = state.shown(key) else {
                    continue;
                };
                let by = shown + origin - place.bounds().position();
                let moved = Transformation::translate(by.x, by.y);
                renderer.with_transformation(moved, |renderer| {
                    piece.as_widget().draw(&tree.children[index], renderer, theme, style, place, shifted(inside, by), &(clip * moved.inverse()));
                });
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);
        let state = tree.state.downcast_ref::<State<K>>();
        let offsets: Vec<Option<Vector>> = self
            .pieces
            .iter()
            .zip(layout.children())
            .map(|((key, _), place)| state.shown(key).map(|at| at + origin - place.bounds().position()))
            .collect();
        let overlays: Vec<_> = self
            .pieces
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .zip(offsets)
            .filter_map(|((((_, piece), child), place), by)| piece.as_widget_mut().overlay(child, place, renderer, viewport, translation + by?))
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: 'a, K: Copy + Eq + Hash + 'static> From<Lanes<'a, Message, K>> for Element<'a, Message> {
    fn from(lanes: Lanes<'a, Message, K>) -> Element<'a, Message> {
        Element::new(lanes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(pairs: &[(u8, usize, f32, f32)]) -> HashMap<u8, (usize, Point)> {
        pairs.iter().map(|(key, lane, x, y)| (*key, (*lane, Point::new(*x, *y)))).collect()
    }

    #[test]
    fn a_panel_moving_to_another_lane_glides_from_where_it_was_seen() {
        let mut state = State::new(vec![0u8, 1]);
        state.replan(vec![Lane::stack(0.0, 100.0, [0u8]), Lane::stack(200.0, 100.0, [1u8])], targets(&[(0, 0, 0.0, 0.0), (1, 1, 200.0, 0.0)]), vec![0.0, 300.0]);
        state.rolls[1].at = 120.0;
        assert_eq!(state.shown(&1), Some(Point::new(200.0, -120.0)));
        state.replan(vec![Lane::stack(0.0, 300.0, [0u8, 1])], targets(&[(0, 0, 0.0, 0.0), (1, 0, 0.0, 140.0)]), vec![0.0]);
        assert_eq!(state.shown(&1), Some(Point::new(200.0, -120.0)), "the panel jumped when the lanes changed");
        let start = Instant::now();
        for frame in 1..=120 {
            state.step(start + std::time::Duration::from_millis(frame * 16));
        }
        assert_eq!(state.shown(&1), Some(Point::new(0.0, 140.0)), "the panel never arrived");
    }

    #[test]
    fn a_lane_scrolls_no_further_than_its_panels() {
        let mut state = State::new(vec![0u8]);
        state.replan(vec![Lane::stack(0.0, 100.0, [0u8])], targets(&[(0, 0, 0.0, 0.0)]), vec![80.0]);
        state.rolls[0].pending = 500.0;
        let start = Instant::now();
        for frame in 1..=60 {
            state.step(start + std::time::Duration::from_millis(frame * 16));
        }
        assert_eq!(state.rolls[0].at, 80.0);
        state.replan(vec![Lane::stack(0.0, 100.0, [0u8])], targets(&[(0, 0, 0.0, 0.0)]), vec![30.0]);
        assert_eq!(state.rolls[0].at, 30.0, "a taller window left the lane scrolled past its end");
    }
}
