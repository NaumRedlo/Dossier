use std::time::Instant;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{operation, tree, Operation, Tree, Widget};
use iced::advanced::{overlay, renderer, Clipboard, Shell};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{mouse, Element, Length, Rectangle, Size, Theme, Vector};

type Renderer = iced::Renderer;

const LINE: f32 = 84.0;
const RATE: f32 = 13.0;

pub struct Glide<'a, Message> {
    content: Element<'a, Message>,
    target: iced::widget::Id,
    aim: Option<(u64, f32)>,
    at: f32,
}

#[derive(Debug, Default)]
struct State {
    pending: f32,
    seen: u64,
    last: Option<Instant>,
}

pub fn glide<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, target: iced::widget::Id) -> Glide<'a, Message> {
    Glide { content: content.into(), target, aim: None, at: 0.0 }
}

impl<Message> Glide<'_, Message> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
