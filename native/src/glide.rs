use std::time::Instant;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{operation, tree, Operation, Tree, Widget};
use iced::advanced::{overlay, renderer, Clipboard, Shell};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{mouse, Element, Length, Rectangle, Size, Theme, Vector};

type Renderer = iced::Renderer;

const LINE: f32 = 84.0;
const RATE: f32 = 13.0;
const SLOP: f32 = 6.0;
const FLING: f32 = 0.22;

pub struct Glide<'a, Message> {
    content: Element<'a, Message>,
    target: iced::widget::Id,
    aim: Option<(u64, f32)>,
    at: f32,
    grabbed: bool,
}

#[derive(Debug, Default)]
struct State {
    pending: f32,
    seen: u64,
    last: Option<Instant>,
    grab: Option<Grab>,
}

#[derive(Debug, Clone, Copy)]
struct Grab {
    from: f32,
    last: f32,
    moved: bool,
    speed: f32,
    at: Instant,
}

pub fn glide<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, target: iced::widget::Id) -> Glide<'a, Message> {
    Glide { content: content.into(), target, aim: None, at: 0.0, grabbed: false }
}

impl<Message> Glide<'_, Message> {
    pub fn grabbed(mut self) -> Self {
        self.grabbed = true;
        self
    }

    pub fn aimed(mut self, aim: Option<(u64, f32)>, at: f32) -> Self {
        self.aim = aim;
        self.at = at;
        self
    }

    fn scroll(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, by: f32) {
        let mut moving = operation::scrollable::scroll_by(self.target.clone(), AbsoluteOffset { x: by, y: 0.0 });
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, &mut moving);
    }
}

fn along(x: f32, y: f32) -> f32 {
    if x.abs() > y.abs() {
        x
    } else {
        y
    }
}

fn eased(pending: f32, dt: f32) -> f32 {
    pending * (1.0 - (-RATE * dt).exp())
}

impl<Message> Widget<Message, Theme, Renderer> for Glide<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        let state = tree.state.downcast_mut::<State>();
        if let Some((serial, x)) = self.aim {
            if serial != state.seen {
                state.seen = serial;
                state.pending = x - self.at;
                state.last = None;
                shell.request_redraw();
            }
        }
        if self.grabbed {
            match event {
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if let Some(point) = cursor.position_over(layout.bounds()) {
                        state.grab = Some(Grab { from: point.x, last: point.x, moved: false, speed: 0.0, at: Instant::now() });
                    }
                }
                iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    if let Some(mut grab) = state.grab {
                        if !grab.moved && (position.x - grab.from).abs() > SLOP {
                            grab.moved = true;
                            grab.last = position.x;
                            state.pending = 0.0;
                        }
                        if grab.moved {
                            let by = grab.last - position.x;
                            let now = Instant::now();
                            let dt = now.saturating_duration_since(grab.at).as_secs_f32().max(0.004);
                            grab.speed = grab.speed * 0.6 + (by / dt) * 0.4;
                            grab.at = now;
                            grab.last = position.x;
                            state.grab = Some(grab);
                            self.scroll(tree, layout, renderer, by);
                            shell.request_redraw();
                            shell.capture_event();
                            return;
                        }
                        state.grab = Some(grab);
                    }
                }
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    if let Some(grab) = state.grab.take() {
                        if grab.moved {
                            let resting = Instant::now().saturating_duration_since(grab.at).as_secs_f32() > 0.08;
                            state.pending = if resting { 0.0 } else { grab.speed * FLING };
                            state.last = None;
                            shell.request_redraw();
                            self.content.as_widget_mut().update(&mut tree.children[0], event, layout, mouse::Cursor::Unavailable, renderer, clipboard, shell, viewport);
                            shell.capture_event();
                            return;
                        }
                    }
                }
                _ => {}
            }
        }
        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(layout.bounds()) => {
                match *delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        state.pending -= along(x, y) * LINE;
                        shell.request_redraw();
                    }
                    mouse::ScrollDelta::Pixels { x, y } => {
                        state.pending = 0.0;
                        self.scroll(tree, layout, renderer, -along(x, y));
                        shell.request_redraw();
                    }
                }
                shell.capture_event();
                return;
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) => {
                let dt = state.last.map_or(1.0 / 60.0, |last| now.saturating_duration_since(last).as_secs_f32()).min(0.05);
                state.last = Some(*now);
                if state.pending.abs() > 0.25 {
                    let step = eased(state.pending, dt);
                    state.pending -= step;
                    self.scroll(tree, layout, renderer, step);
                    shell.request_redraw();
                } else {
                    state.pending = 0.0;
                    state.last = None;
                }
            }
            _ => {}
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        if tree.state.downcast_ref::<State>().grab.is_some_and(|grab| grab.moved) {
            return mouse::Interaction::Grabbing;
        }
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
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
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Glide<'a, Message>> for Element<'a, Message> {
    fn from(glide: Glide<'a, Message>) -> Element<'a, Message> {
        Element::new(glide)
    }
}

const EDGE: f32 = 64.0;
const EDGE_SPEED: f32 = 900.0;

pub struct Edged<'a, Message> {
    content: Element<'a, Message>,
    target: iced::widget::Id,
}

#[derive(Debug, Default)]
struct EdgeState {
    held: bool,
    speed: f32,
    last: Option<Instant>,
}

pub fn edged<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, target: iced::widget::Id) -> Edged<'a, Message> {
    Edged { content: content.into(), target }
}

fn edge_speed(bounds: Rectangle, y: f32) -> f32 {
    let (top, bottom) = (y - bounds.y, bounds.y + bounds.height - y);
    let pull = |gap: f32| {
        let k = 1.0 - gap.max(0.0) / EDGE;
        EDGE_SPEED * k * k
    };
    if top < EDGE {
        -pull(top)
    } else if bottom < EDGE {
        pull(bottom)
    } else {
        0.0
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Edged<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<EdgeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(EdgeState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        let state = tree.state.downcast_mut::<EdgeState>();
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => state.held = true,
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.held = false;
                state.speed = 0.0;
                state.last = None;
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) if state.speed != 0.0 => {
                let dt = crate::ui::elapsed(&mut state.last, *now);
                let by = state.speed * dt;
                let mut moving = operation::scrollable::scroll_by(self.target.clone(), AbsoluteOffset { x: 0.0, y: by });
                self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, &mut moving);
                shell.request_redraw();
            }
            _ => {}
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        let state = tree.state.downcast_mut::<EdgeState>();
        if let iced::Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            let dragging = state.held && shell.is_event_captured();
            state.speed = if dragging { edge_speed(layout.bounds(), position.y) } else { 0.0 };
            if state.speed != 0.0 {
                shell.request_redraw();
            } else {
                state.last = None;
            }
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
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
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Edged<'a, Message>> for Element<'a, Message> {
    fn from(edged: Edged<'a, Message>) -> Element<'a, Message> {
        Element::new(edged)
    }
}

pub const BRIM: f32 = 26.0;

const BRIM_SOON: f32 = 8.0;
const BRIM_STOPS: [(f32, f32); 7] = [(0.0, 1.0), (0.15, 0.961), (0.3, 0.84), (0.45, 0.656), (0.6, 0.439), (0.8, 0.16), (1.0, 0.0)];

pub fn brim_strength(under: f32) -> f32 {
    (under.abs() / BRIM_SOON).clamp(0.0, 1.0)
}

pub fn draw_brim(renderer: &mut Renderer, top: Rectangle, colour: iced::Color, k: f32) {
    draw_edge(renderer, top, colour, k, false);
}

pub fn draw_edge(renderer: &mut Renderer, edge: Rectangle, colour: iced::Color, k: f32, rising: bool) {
    use iced::advanced::Renderer as _;
    if k <= 0.003 || edge.width <= 0.0 || edge.height <= 0.0 {
        return;
    }
    let angle = if rising { 0.0 } else { std::f32::consts::PI };
    let shade = BRIM_STOPS.iter().fold(iced::gradient::Linear::new(iced::Radians(angle)), |shade, (at, a)| shade.add_stop(*at, iced::Color { a: a * k, ..colour }));
    renderer.with_layer(edge, |renderer| {
        renderer.fill_quad(renderer::Quad { bounds: edge, ..renderer::Quad::default() }, iced::Background::Gradient(shade.into()));
    });
}

pub struct Brim<'a, Message> {
    content: Element<'a, Message>,
    colour: iced::Color,
    fade: f32,
    foot: bool,
    high: f32,
}

#[derive(Debug, Default)]
struct BrimState {
    under: f32,
    ahead: f32,
}

pub fn brim<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Brim<'a, Message> {
    Brim { content: content.into(), colour: crate::theme::GROUND, fade: crate::ui::fade(), foot: false, high: BRIM }
}

impl<Message> Brim<'_, Message> {
    pub fn on(mut self, colour: iced::Color) -> Self {
        self.colour = colour;
        self
    }

    pub fn both(mut self) -> Self {
        self.foot = true;
        self
    }

    pub fn high(mut self, high: f32) -> Self {
        self.high = high;
        self
    }
}

pub fn ahead(viewport: f32, content: f32, under: f32) -> f32 {
    (content - viewport - under).max(0.0)
}

struct Scrolled(Option<(f32, f32)>);

impl Operation for Scrolled {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if self.0.is_none() {
            operate(self);
        }
    }

    fn scrollable(&mut self, _: Option<&iced::widget::Id>, bounds: Rectangle, content: Rectangle, translation: Vector, _: &mut dyn operation::Scrollable) {
        if self.0.is_none() {
            self.0 = Some((translation.y, ahead(bounds.height, content.height, translation.y)));
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Brim<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<BrimState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(BrimState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        if let iced::Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            let mut found = Scrolled(None);
            self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, &mut found);
            let (under, ahead) = found.0.unwrap_or((0.0, 0.0));
            let state = tree.state.downcast_mut::<BrimState>();
            state.under = under;
            state.ahead = ahead;
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<BrimState>();
        let high = self.high.min(bounds.height);
        draw_brim(renderer, Rectangle { height: high, ..bounds }, self.colour, brim_strength(state.under) * self.fade);
        if self.foot {
            draw_edge(renderer, Rectangle { y: bounds.y + bounds.height - high, height: high, ..bounds }, self.colour, brim_strength(state.ahead) * self.fade, true);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Brim<'a, Message>> for Element<'a, Message> {
    fn from(brim: Brim<'a, Message>) -> Element<'a, Message> {
        Element::new(brim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_top_edge_fades_only_once_something_has_scrolled_under_it() {
        assert_eq!(brim_strength(0.0), 0.0);
        assert!(brim_strength(4.0) > 0.4 && brim_strength(4.0) < 0.6);
        assert_eq!(brim_strength(400.0), 1.0);
        assert_eq!(brim_strength(-400.0), 1.0);
        assert!(BRIM_STOPS.windows(2).all(|pair| pair[0].0 < pair[1].0 && pair[0].1 > pair[1].1), "the fade does not thin out steadily");
        assert!(BRIM_STOPS[0].1 == 1.0 && BRIM_STOPS[BRIM_STOPS.len() - 1].1 == 0.0);
    }

    #[test]
    fn the_bottom_edge_fades_while_something_is_still_below() {
        assert_eq!(ahead(200.0, 500.0, 0.0), 300.0);
        assert_eq!(ahead(200.0, 500.0, 300.0), 0.0);
        assert_eq!(ahead(200.0, 120.0, 0.0), 0.0);
        assert_eq!(brim_strength(ahead(200.0, 500.0, 296.0)), 0.5);
    }

    #[test]
    fn a_wheel_notch_is_spread_over_frames_and_arrives_whole() {
        let mut pending = LINE;
        let mut travelled = 0.0;
        let mut frames = 0;
        let mut largest = 0.0f32;
        while pending.abs() > 0.25 {
            let step = eased(pending, 1.0 / 120.0);
            pending -= step;
            travelled += step;
            largest = largest.max(step);
            frames += 1;
        }
        assert!(frames > 20, "{frames} frames is still a jump");
        assert!(largest < LINE / 5.0, "one frame carried {largest} of {LINE}");
        assert!((travelled - LINE).abs() < 0.3);
    }

    #[test]
    fn holding_near_an_edge_scrolls_harder_the_closer_it_gets() {
        let bounds = Rectangle::new(iced::Point::new(0.0, 100.0), Size::new(500.0, 400.0));
        assert_eq!(edge_speed(bounds, 300.0), 0.0);
        assert!(edge_speed(bounds, 110.0) < edge_speed(bounds, 150.0) && edge_speed(bounds, 150.0) < 0.0);
        assert!(edge_speed(bounds, 495.0) > edge_speed(bounds, 460.0) && edge_speed(bounds, 460.0) > 0.0);
    }

    #[test]
    fn a_vertical_wheel_moves_a_row_sideways() {
        assert_eq!(along(0.0, -2.0), -2.0);
        assert_eq!(along(3.0, 1.0), 3.0);
    }
}
