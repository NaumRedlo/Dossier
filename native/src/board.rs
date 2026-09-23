use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{overlay, renderer, Clipboard, Shell};
use iced::{mouse, Background, Border, Color, Element, Length, Point, Rectangle, Shadow, Size, Theme, Transformation, Vector};

type Renderer = iced::Renderer;

const SLACK: f32 = 6.0;
const DWELL: Duration = Duration::from_millis(50);
const SHOWN_FOR: Duration = Duration::from_millis(700);
const RADIUS: f32 = 16.0;

#[derive(Debug, Clone, Copy, Default)]
struct Spring {
    x: f32,
    v: f32,
}

#[derive(Debug, Clone, Copy)]
struct Feel {
    omega: f32,
    zeta: f32,
}

const FLOW: Feel = Feel { omega: 15.0, zeta: 0.92 };
const FOLLOW: Feel = Feel { omega: 26.0, zeta: 0.8 };
const LAND: Feel = Feel { omega: 17.0, zeta: 0.86 };
const LIFT: Feel = Feel { omega: 16.0, zeta: 1.0 };
const FADE: Feel = Feel { omega: 13.0, zeta: 1.0 };

impl Spring {
    fn at(x: f32) -> Self {
        Spring { x, v: 0.0 }
    }

    fn step(&mut self, target: f32, dt: f32, feel: Feel) -> bool {
        let slices = (dt / (1.0 / 240.0)).ceil().max(1.0) as usize;
        let h = dt / slices as f32;
        for _ in 0..slices {
            let pull = feel.omega * feel.omega * (target - self.x) - 2.0 * feel.zeta * feel.omega * self.v;
            self.v += pull * h;
            self.x += self.v * h;
        }
        let resting = (target - self.x).abs() < 0.05 && self.v.abs() < 0.5;
        if resting {
            self.x = target;
            self.v = 0.0;
        }
        !resting
    }

    fn step_fraction(&mut self, target: f32, dt: f32, feel: Feel) -> bool {
        let slices = (dt / (1.0 / 240.0)).ceil().max(1.0) as usize;
        let h = dt / slices as f32;
        for _ in 0..slices {
            let pull = feel.omega * feel.omega * (target - self.x) - 2.0 * feel.zeta * feel.omega * self.v;
            self.v += pull * h;
            self.x += self.v * h;
        }
        let resting = (target - self.x).abs() < 0.001 && self.v.abs() < 0.01;
        if resting {
            self.x = target;
            self.v = 0.0;
        }
        !resting
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Spot {
    x: Spring,
    y: Spring,
}

impl Spot {
    fn at(p: Point) -> Self {
        Spot { x: Spring::at(p.x), y: Spring::at(p.y) }
    }

    fn point(&self) -> Point {
        Point::new(self.x.x, self.y.x)
    }

    fn step(&mut self, target: Point, dt: f32, feel: Feel) -> bool {
        let a = self.x.step(target.x, dt, feel);
        let b = self.y.step(target.y, dt, feel);
        a || b
    }
}

#[derive(Debug, Clone, Copy)]
struct Press<K> {
    key: K,
    from: Point,
    grab: Vector,
}

#[derive(Debug, Clone, Copy)]
struct Held<K> {
    key: K,
    grab: Vector,
    pointer: Point,
    screen: Point,
    released: bool,
    lift: Spring,
}

struct State<K> {
    keys: Vec<K>,
    sizes: HashMap<K, Size>,
    width: f32,
    targets: HashMap<K, Point>,
    spots: HashMap<K, Spot>,
    order: Option<Vec<K>>,
    press: Option<Press<K>>,
    held: Option<Held<K>>,
    pending: Option<(Vec<K>, Instant)>,
    frame: Spot,
    frame_seen: Spring,
    shown: Spring,
    showing: bool,
    shown_until: Option<Instant>,
    last: Option<Instant>,
}

impl<K: Copy + Eq + Hash> State<K> {
    fn new(keys: Vec<K>) -> Self {
        State {
            keys,
            sizes: HashMap::new(),
            width: 0.0,
            targets: HashMap::new(),
            spots: HashMap::new(),
            order: None,
            press: None,
            held: None,
            pending: None,
            frame: Spot::default(),
            frame_seen: Spring::default(),
            shown: Spring::default(),
            showing: false,
            shown_until: None,
            last: None,
        }
    }

    fn shown_order(&self) -> Vec<K> {
        match &self.order {
            Some(order) if order.len() == self.keys.len() && self.keys.iter().all(|k| order.contains(k)) => order.clone(),
            _ => self.keys.clone(),
        }
    }

    fn step(&mut self, now: Instant) -> bool {
        let dt = match self.last {
            Some(last) if now < last => return true,
            Some(last) => now.duration_since(last).as_secs_f32().min(0.05),
            None => 1.0 / 60.0,
        };
        self.last = Some(now);
        let mut moving = false;
        let held = self.held.map(|held| held.key);
        for (key, spot) in self.spots.iter_mut() {
            let Some(target) = self.targets.get(key).copied() else {
                continue;
            };
            if Some(*key) == held {
                continue;
            }
            moving |= spot.step(target, dt, FLOW);
        }
        if let Some(held) = self.held.as_mut() {
            let slot = self.targets.get(&held.key).copied().unwrap_or(Point::ORIGIN);
            let (aim, feel) = match held.released {
                false => (held.pointer - held.grab, FOLLOW),
                true => (slot, LAND),
            };
            let lifted = if held.released { 0.0 } else { 1.0 };
            if let Some(spot) = self.spots.get_mut(&held.key) {
                moving |= spot.step(aim, dt, feel);
            }
            moving |= held.lift.step_fraction(lifted, dt, LIFT);
            moving |= self.frame.step(slot, dt, FLOW);
            moving |= self.frame_seen.step_fraction(if held.released { 0.0 } else { 1.0 }, dt, FADE);
            let landed = held.released
                && self.spots.get(&held.key).is_none_or(|spot| (spot.point() - slot).x.abs() < 0.05 && (spot.point() - slot).y.abs() < 0.05)
                && held.lift.x <= 0.0005
                && self.frame_seen.x <= 0.0005;
            if landed {
                self.held = None;
            } else {
                moving = true;
            }
        }
        if self.shown_until.is_some_and(|until| now >= until) {
            self.shown_until = None;
            self.showing = false;
        }
        moving |= self.shown.step_fraction(if self.showing { 1.0 } else { 0.0 }, dt, FADE);
        moving || self.shown_until.is_some()
    }
}

fn flow<K: Copy + Eq + Hash>(order: &[K], sizes: &HashMap<K, Size>, width: f32, spacing: f32) -> (HashMap<K, Point>, f32) {
    let mut at = HashMap::with_capacity(order.len());
    let (mut x, mut y, mut row) = (0.0f32, 0.0f32, 0.0f32);
    for key in order {
        let size = sizes.get(key).copied().unwrap_or(Size::ZERO);
        if x > 0.0 && x + size.width > width {
            x = 0.0;
            y += row + spacing;
            row = 0.0;
        }
        at.insert(*key, Point::new(x, y));
        x += size.width + spacing;
        row = row.max(size.height);
    }
    let height = if order.is_empty() { 0.0 } else { y + row };
    (at, height)
}

fn aimed<K: Copy + Eq + Hash>(order: &[K], held: K, targets: &HashMap<K, Point>, sizes: &HashMap<K, Size>, pointer: Point) -> Option<Vec<K>> {
    let others: Vec<K> = order.iter().copied().filter(|k| *k != held).collect();
    let rect = |key: &K| Some(Rectangle::new(*targets.get(key)?, *sizes.get(key)?));
    let placed = |at: usize| {
        let mut next = others.clone();
        next.insert(at.min(next.len()), held);
        next
    };
    if rect(&held).is_some_and(|own| own.contains(pointer)) {
        return None;
    }
    for (at, key) in others.iter().enumerate() {
        let Some(bounds) = rect(key) else {
            continue;
        };
        if bounds.contains(pointer) {
            let after = pointer.x > bounds.center_x();
            return Some(placed(at + usize::from(after)));
        }
    }
    let row_end = others
        .iter()
        .enumerate()
        .filter_map(|(at, key)| rect(key).map(|bounds| (at, bounds)))
        .filter(|(_, bounds)| pointer.y >= bounds.y && pointer.y <= bounds.y + bounds.height && pointer.x > bounds.x + bounds.width)
        .max_by(|a, b| (a.1.x + a.1.width).total_cmp(&(b.1.x + b.1.width)));
    if let Some((at, _)) = row_end {
        return Some(placed(at + 1));
    }
    let bottom = others.iter().filter_map(|key| rect(key)).map(|bounds| bounds.y + bounds.height).fold(0.0f32, f32::max);
    (pointer.y > bottom && !others.is_empty()).then(|| placed(others.len()))
}

fn aim_at<K: Copy + Eq + Hash>(state: &mut State<K>, origin: Vector) {
    let Some(held) = state.held.as_mut() else {
        return;
    };
    held.pointer = held.screen - origin;
    let (key, pointer) = (held.key, held.pointer);
    let order = state.shown_order();
    match aimed(&order, key, &state.targets, &state.sizes, pointer) {
        Some(next) if next != order => {
            if state.pending.as_ref().is_none_or(|(waiting, _)| *waiting != next) {
                let since = state.last.unwrap_or_else(Instant::now);
                state.pending = Some((next, since));
            }
        }
        Some(_) => state.pending = None,
        None => {}
    }
}

fn shifted(cursor: mouse::Cursor, by: Vector) -> mouse::Cursor {
    match cursor {
        mouse::Cursor::Available(p) => mouse::Cursor::Available(p - by),
        other => other,
    }
}

fn about(bounds: Rectangle, scale: f32) -> Transformation {
    let (cx, cy) = (bounds.center_x(), bounds.center_y());
    Transformation::translate(cx, cy) * Transformation::scale(scale) * Transformation::translate(-cx, -cy)
}

fn outline(renderer: &mut Renderer, bounds: Rectangle, radius: f32, width: f32, colour: Color) {
    use iced::advanced::Renderer as _;
    if colour.a <= 0.002 {
        return;
    }
    renderer.fill_quad(
        renderer::Quad { bounds, border: Border { color: colour, width, radius: radius.into() }, ..renderer::Quad::default() },
        Background::Color(Color::TRANSPARENT),
    );
}

pub struct Board<'a, Message, K> {
    pieces: Vec<(K, Element<'a, Message>)>,
    spacing: f32,
    on_move: Box<dyn Fn(K, Option<K>) -> Message + 'a>,
    fade: f32,
    solid: Option<Color>,
}

pub fn board<'a, Message: 'a, K: Copy + Eq + Hash + 'static>(
    pieces: Vec<(K, Element<'a, Message>)>,
    spacing: f32,
    on_move: impl Fn(K, Option<K>) -> Message + 'a,
) -> Board<'a, Message, K> {
    Board { pieces, spacing, on_move: Box::new(on_move), fade: crate::ui::fade(), solid: None }
}

impl<Message, K: Copy + Eq + Hash + 'static> Board<'_, Message, K> {
    pub fn solid(mut self, colour: Color) -> Self {
        self.solid = Some(colour);
        self
    }

    fn backing(&self, renderer: &mut Renderer, bounds: Rectangle) {
        use iced::advanced::Renderer as _;
        let Some(colour) = self.solid else {
            return;
        };
        renderer.fill_quad(
            renderer::Quad { bounds, border: Border { radius: RADIUS.into(), ..Border::default() }, ..renderer::Quad::default() },
            Background::Color(Color { a: colour.a * self.fade, ..colour }),
        );
    }

    fn keys(&self) -> Vec<K> {
        self.pieces.iter().map(|(key, _)| *key).collect()
    }
}

impl<Message, K: Copy + Eq + Hash + 'static> Widget<Message, Theme, Renderer> for Board<'_, Message, K> {
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
        let before = std::mem::replace(&mut state.keys, keys.clone());
        if state.order.as_ref().is_some_and(|order| *order == keys) && state.held.is_none_or(|held| held.released) {
            state.order = None;
        }
        if before.len() != keys.len() || !keys.iter().all(|k| before.contains(k)) {
            state.order = None;
            state.held = None;
            state.press = None;
        }
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
        Size { width: Length::Fill, height: Length::Shrink }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let width = limits.max().width;
        let loose = layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
        let nodes: Vec<Node> = self
            .pieces
            .iter_mut()
            .zip(tree.children.iter_mut())
            .map(|((_, piece), child)| piece.as_widget_mut().layout(child, renderer, &loose))
            .collect();
        let state = tree.state.downcast_mut::<State<K>>();
        state.sizes = self.pieces.iter().zip(&nodes).map(|((key, _), node)| (*key, node.size())).collect();
        state.width = width;
        let (targets, height) = flow(&state.shown_order(), &state.sizes, width, self.spacing);
        for (key, target) in &targets {
            state.spots.entry(*key).or_insert_with(|| Spot::at(*target));
        }
        if state.held.is_none() {
            state.frame = Spot::default();
        }
        state.targets = targets;
        let placed = self
            .pieces
            .iter()
            .zip(nodes)
            .map(|((key, _), node)| node.move_to(state.targets.get(key).copied().unwrap_or(Point::ORIGIN)))
            .collect();
        Node::with_children(Size::new(width, height), placed)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        for (((_, piece), child), place) in self.pieces.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            piece.as_widget_mut().operate(child, place, renderer, operation);
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
        let origin = layout.bounds().position();
        let origin_v = Vector::new(origin.x, origin.y);
        let state = tree.state.downcast_mut::<State<K>>();

        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if state.held.is_some_and(|held| !held.released) {
                if let (Some(held), Some(at)) = (state.held.as_mut(), cursor.land().position()) {
                    held.screen = at;
                }
                aim_at(state, origin_v);
            }
            if state.step(*now) {
                shell.request_redraw();
            }
            if state.held.is_some_and(|held| !held.released) && state.pending.as_ref().is_some_and(|(_, since)| now.saturating_duration_since(*since) >= DWELL) {
                state.order = state.pending.take().map(|(next, _)| next);
                shell.invalidate_layout();
                shell.request_redraw();
            }
        }

        let dragging = state.held.is_some_and(|held| !held.released);
        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) if dragging => {
                if let (Some(held), Some(at)) = (state.held.as_mut(), cursor.land().position()) {
                    held.screen = at;
                }
                aim_at(state, origin_v);
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if dragging => {
                state.pending = None;
                let order = state.shown_order();
                if let Some(held) = state.held.as_mut() {
                    held.released = true;
                    if order != state.keys {
                        let before = order.iter().position(|k| *k == held.key).and_then(|at| order.get(at + 1).copied());
                        shell.publish((self.on_move)(held.key, before));
                    }
                }
                state.press = None;
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some(press), Some(position)) = (state.press, cursor.land().position()) {
                    if (position.x - press.from.x).abs() + (position.y - press.from.y).abs() > SLACK {
                        state.press = None;
                        let pointer = position - origin_v;
                        state.held = Some(Held { key: press.key, grab: press.grab, pointer, screen: position, released: false, lift: Spring::default() });
                        state.order = Some(state.shown_order());
                        let slot = state.targets.get(&press.key).copied().unwrap_or(Point::ORIGIN);
                        state.frame = Spot::at(slot);
                        state.frame_seen = Spring::default();
                        state.last = None;
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                }
            }
            _ => {}
        }

        let held = state.held.map(|held| held.key);
        let spots: HashMap<K, Point> = state.spots.iter().map(|(k, s)| (*k, s.point())).collect();
        for (((key, piece), child), place) in self.pieces.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            if Some(*key) == held && !matches!(event, iced::Event::Window(_)) {
                continue;
            }
            let shown = spots.get(key).map_or(place.bounds().position(), |p| *p + origin_v);
            let by = shown - place.bounds().position();
            piece.as_widget_mut().update(child, event, place, shifted(cursor, by), renderer, clipboard, shell, viewport);
        }

        let state = tree.state.downcast_mut::<State<K>>();
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if !shell.is_event_captured() && state.held.is_none() => {
                let Some(at) = cursor.position() else {
                    return;
                };
                let hit = self.pieces.iter().find_map(|(key, _)| {
                    let spot = state.spots.get(key)?.point();
                    let size = state.sizes.get(key)?;
                    let rect = Rectangle::new(spot + origin_v, *size);
                    rect.contains(at).then_some((*key, rect))
                });
                match hit {
                    Some((key, rect)) => {
                        state.press = Some(Press { key, from: at, grab: at - rect.position() });
                    }
                    None if layout.bounds().contains(at) => {
                        state.showing = true;
                        state.shown_until = None;
                        shell.request_redraw();
                    }
                    None => {}
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.press = None;
                if state.showing && state.shown_until.is_none() {
                    state.shown_until = Some(Instant::now() + SHOWN_FOR);
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State<K>>();
        if state.held.is_some_and(|held| !held.released) {
            return mouse::Interaction::Grabbing;
        }
        let origin = layout.bounds().position();
        let origin_v = Vector::new(origin.x, origin.y);
        self.pieces
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|(((key, piece), child), place)| {
                let shown = state.spots.get(key).map_or(place.bounds().position(), |s| s.point() + origin_v);
                let by = shown - place.bounds().position();
                piece.as_widget().mouse_interaction(child, place, shifted(cursor, by), viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<State<K>>();
        let origin = layout.bounds().position();
        let origin_v = Vector::new(origin.x, origin.y);
        let shown = state.shown.x.clamp(0.0, 1.0);
        let fade = self.fade;
        let held = state.held;

        if shown > 0.0 {
            for (key, _) in &self.pieces {
                let (Some(target), Some(size)) = (state.targets.get(key), state.sizes.get(key)) else {
                    continue;
                };
                let slot = Rectangle::new(*target + origin_v, *size).expand(4.0);
                outline(renderer, slot, RADIUS + 4.0, 1.0, Color::from_rgba(1.0, 1.0, 1.0, 0.2 * shown * fade));
            }
        }

        if let Some(held) = held {
            if let Some(size) = state.sizes.get(&held.key) {
                let seen = state.frame_seen.x.clamp(0.0, 1.0);
                let frame = Rectangle::new(state.frame.point() + origin_v, *size);
                outline(renderer, frame, RADIUS, 1.5, Color::from_rgba(1.0, 1.0, 1.0, 0.22 * seen * fade));
            }
        }

        for (((key, piece), child), place) in self.pieces.iter().zip(tree.children.iter()).zip(layout.children()) {
            if held.is_some_and(|held| held.key == *key) {
                continue;
            }
            let at = state.spots.get(key).map_or(place.bounds().position(), |s| s.point() + origin_v);
            let by = at - place.bounds().position();
            let calm = 1.0 - 0.025 * shown;
            let transformation = Transformation::translate(by.x, by.y) * about(place.bounds(), calm);
            let travelling = state.targets.get(key).is_some_and(|target| {
                let off = at - (*target + origin_v);
                off.x.abs() > 0.5 || off.y.abs() > 0.5
            });
            renderer.with_transformation(transformation, |renderer| {
                if travelling {
                    self.backing(renderer, place.bounds());
                }
                piece.as_widget().draw(child, renderer, theme, style, place, shifted(cursor, by), viewport);
            });
        }

        let Some(held) = held else {
            return;
        };
        let found = self.pieces.iter().zip(tree.children.iter()).zip(layout.children()).find(|(((key, _), _), _)| *key == held.key);
        let Some((((_, piece), child), place)) = found else {
            return;
        };
        let lift = held.lift.x.clamp(0.0, 1.2);
        let at = state.spots.get(&held.key).map_or(place.bounds().position(), |s| s.point() + origin_v);
        let by = at - place.bounds().position();
        let lifted = Rectangle::new(at, place.bounds().size());
        let scale = 1.0 + 0.035 * lift;
        renderer.with_layer(*viewport, |renderer| {
            renderer.with_transformation(about(lifted, scale), |renderer| {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: lifted,
                        border: Border { radius: RADIUS.into(), ..Border::default() },
                        shadow: Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.55 * lift * fade),
                            offset: Vector::new(0.0, 6.0 + 10.0 * lift),
                            blur_radius: 12.0 + 28.0 * lift,
                        },
                        ..renderer::Quad::default()
                    },
                    Background::Color(Color::TRANSPARENT),
                );
                self.backing(renderer, lifted);
            });
            let transformation = about(lifted, scale) * Transformation::translate(by.x, by.y);
            renderer.with_transformation(transformation, |renderer| {
                piece.as_widget().draw(child, renderer, theme, style, place, mouse::Cursor::Unavailable, viewport);
            });
            renderer.with_transformation(about(lifted, scale), |renderer| {
                outline(renderer, lifted, RADIUS, 1.0, Color::from_rgba(1.0, 1.0, 1.0, 0.13 * lift * fade));
            });
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
        let overlays: Vec<_> = self
            .pieces
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .filter_map(|(((_, piece), child), place)| piece.as_widget_mut().overlay(child, place, renderer, viewport, translation))
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: 'a, K: Copy + Eq + Hash + 'static> From<Board<'a, Message, K>> for Element<'a, Message> {
    fn from(board: Board<'a, Message, K>) -> Element<'a, Message> {
        Element::new(board)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spring_arrives_without_a_jump_and_comes_to_rest() {
        let mut spring = Spring::at(0.0);
        let mut last = 0.0;
        let mut largest_step = 0.0f32;
        let mut frames = 0;
        while spring.step(100.0, 1.0 / 120.0, FLOW) {
            largest_step = largest_step.max((spring.x - last).abs());
            last = spring.x;
            frames += 1;
            assert!(frames < 600, "the spring never settled");
        }
        assert_eq!(spring.x, 100.0);
        assert!(largest_step < 12.0, "a step of {largest_step} is a jump, not a glide");
        assert!(frames > 20, "{frames} frames is a snap");
    }

    #[test]
    fn the_flow_wraps_like_the_grid_it_replaces() {
        let sizes: HashMap<u8, Size> = [(1, Size::new(300.0, 100.0)), (2, Size::new(300.0, 80.0)), (3, Size::new(300.0, 50.0))].into();
        let (at, height) = flow(&[1, 2, 3], &sizes, 700.0, 10.0);
        assert_eq!(at[&1], Point::new(0.0, 0.0));
        assert_eq!(at[&2], Point::new(310.0, 0.0));
        assert_eq!(at[&3], Point::new(0.0, 110.0));
        assert_eq!(height, 160.0);
    }

    #[test]
    fn the_pointer_says_where_a_held_tile_goes() {
        let sizes: HashMap<u8, Size> = [(1, Size::new(300.0, 100.0)), (2, Size::new(300.0, 100.0)), (3, Size::new(200.0, 100.0))].into();
        let order = [1, 2, 3];
        let (targets, _) = flow(&order, &sizes, 700.0, 10.0);
        assert_eq!(aimed(&order, 1, &targets, &sizes, Point::new(100.0, 50.0)), None, "over its own place it stays");
        assert_eq!(aimed(&order, 3, &targets, &sizes, Point::new(40.0, 50.0)), Some(vec![3, 1, 2]), "the left half of a tile puts it before");
        assert_eq!(aimed(&order, 1, &targets, &sizes, Point::new(560.0, 50.0)), Some(vec![2, 1, 3]), "the right half puts it after");
        assert_eq!(aimed(&order, 1, &targets, &sizes, Point::new(650.0, 150.0)), Some(vec![2, 3, 1]), "the empty end of a row puts it last in the row");
        assert_eq!(aimed(&order, 3, &targets, &sizes, Point::new(100.0, 400.0)), Some(vec![1, 2, 3]), "below everything puts it last");
        assert_eq!(aimed(&order, 1, &targets, &sizes, Point::new(305.0, 50.0)), None, "the gap between tiles changes nothing");
    }
}
