use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{renderer, Clipboard, Shell};
use iced::{mouse, Element, Length, Point, Rectangle, Size, Theme, Vector};

type Renderer = iced::Renderer;

const GLIDE: Duration = Duration::from_millis(700);

pub struct Ticker<'a, Message> {
    items: Vec<Element<'a, Message>>,
    gap: f32,
    dwell: Duration,
}

pub fn ticker<'a, Message: 'a>(items: Vec<Element<'a, Message>>, gap: f32, dwell: Duration) -> Ticker<'a, Message> {
    Ticker { items, gap, dwell }
}

#[derive(Debug, Default)]
struct State {
    tops: Vec<f32>,
    period: f32,
    moving: bool,
    at: usize,
    offset: f32,
    from: f32,
    to: f32,
    since: Option<Instant>,
    next: Option<Instant>,
    over: bool,
}

fn eased(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

impl State {
    fn step(&mut self, now: Instant, dwell: Duration) -> Option<Instant> {
        if !self.moving {
            return None;
        }
        if let Some(since) = self.since {
            let k = now.saturating_duration_since(since).as_secs_f32() / GLIDE.as_secs_f32();
            self.offset = self.from + (self.to - self.from) * eased(k);
            if k < 1.0 {
                return Some(now);
            }
            self.offset = self.to;
            if self.at >= self.tops.len() {
                self.at = 0;
                self.offset -= self.period;
            }
            self.since = None;
            self.next = Some(now + dwell);
            return self.next;
        }
        if self.over {
            self.next = Some(now + dwell);
            return self.next;
        }
        match self.next {
            Some(next) if now >= next => {
                self.from = self.offset;
                self.at += 1;
                self.to = self.tops.get(self.at).copied().unwrap_or(self.period);
                self.since = Some(now);
                Some(now)
            }
            Some(next) => Some(next),
            None => {
                self.next = Some(now + dwell);
                self.next
            }
        }
    }

    fn shift_for(&self, y: f32) -> f32 {
        if self.moving && y + self.offset >= self.period {
            self.offset - self.period
        } else {
            self.offset
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Ticker<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.items.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.items);
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let bounds = limits.max();
        let loose = layout::Limits::new(Size::ZERO, Size::new(bounds.width, f32::INFINITY));
        let mut y = 0.0;
        let mut tops = Vec::with_capacity(self.items.len());
        let nodes: Vec<Node> = self
            .items
            .iter_mut()
            .zip(tree.children.iter_mut())
            .map(|(item, child)| {
                let node = item.as_widget_mut().layout(child, renderer, &loose).move_to(Point::new(0.0, y));
                tops.push(y);
                y += node.size().height + self.gap;
                node
            })
            .collect();
        let state = tree.state.downcast_mut::<State>();
        let fits = y - self.gap <= bounds.height + 0.5;
        state.moving = !fits && tops.len() > 1;
        state.period = y;
        state.tops = tops;
        if state.at >= state.tops.len() {
            state.at = 0;
            state.offset = 0.0;
            state.since = None;
        }
        if !state.moving {
            state.offset = 0.0;
            state.at = 0;
            state.since = None;
        }
        Node::with_children(bounds, nodes)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        for ((item, child), place) in self.items.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            item.as_widget_mut().operate(child, place, renderer, operation);
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
        let state = tree.state.downcast_mut::<State>();
        match event {
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) => {
                if let Some(at) = state.step(*now, self.dwell) {
                    shell.request_redraw_at(iced::window::RedrawRequest::At(at));
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) | iced::Event::Mouse(mouse::Event::CursorLeft) => {
                let over = cursor.is_over(bounds);
                if over != state.over {
                    state.over = over;
                    shell.request_redraw();
                }
            }
            _ => {}
        }
        let inside = cursor.position().filter(|at| bounds.contains(*at));
        let shift = inside.map_or(state.offset, |at| state.shift_for(at.y - bounds.y));
        let cursor = match inside {
            Some(at) => mouse::Cursor::Available(at + Vector::new(0.0, shift)),
            None => mouse::Cursor::Unavailable,
        };
        for ((item, child), place) in self.items.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            item.as_widget_mut().update(child, event, place, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let Some(at) = cursor.position().filter(|at| bounds.contains(*at)) else {
            return mouse::Interaction::default();
        };
        let cursor = mouse::Cursor::Available(at + Vector::new(0.0, state.shift_for(at.y - bounds.y)));
        self.items
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((item, child), place)| item.as_widget().mouse_interaction(child, place, cursor, viewport, renderer))
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
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let copies: &[f32] = if state.moving { &[0.0, 1.0] } else { &[0.0] };
        renderer.with_layer(bounds, |renderer| {
            for copy in copies {
                let shift = copy * state.period - state.offset;
                renderer.with_translation(Vector::new(0.0, shift), |renderer| {
                    for ((item, child), place) in self.items.iter().zip(tree.children.iter()).zip(layout.children()) {
                        let top = place.bounds().y + shift;
                        if top > bounds.y + bounds.height || top + place.bounds().height < bounds.y {
                            continue;
                        }
                        item.as_widget().draw(child, renderer, theme, style, place, cursor, viewport);
                    }
                });
            }
        });
    }
}

impl<'a, Message: 'a> From<Ticker<'a, Message>> for Element<'a, Message> {
    fn from(ticker: Ticker<'a, Message>) -> Element<'a, Message> {
        Element::new(ticker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolling() -> State {
        State { tops: vec![0.0, 50.0, 100.0], period: 150.0, moving: true, ..State::default() }
    }

    #[test]
    fn a_ticker_waits_then_glides_one_item_and_waits_again() {
        let mut state = rolling();
        let start = Instant::now();
        let dwell = Duration::from_secs(4);
        assert_eq!(state.step(start, dwell), Some(start + dwell));
        let go = start + dwell;
        assert_eq!(state.step(go, dwell), Some(go));
        let halfway = go + GLIDE / 2;
        state.step(halfway, dwell);
        assert!(state.offset > 0.0 && state.offset < 50.0, "halfway is between two items: {}", state.offset);
        let landed = go + GLIDE;
        assert_eq!(state.step(landed, dwell), Some(landed + dwell));
        assert_eq!(state.offset, 50.0);
    }

    #[test]
    fn after_the_last_item_the_first_comes_round_again() {
        let mut state = rolling();
        let dwell = Duration::from_secs(1);
        let mut now = Instant::now();
        for _ in 0..40 {
            now += Duration::from_millis(100);
            state.step(now, dwell);
            assert!((0.0..=150.0).contains(&state.offset), "the offset left the loop: {}", state.offset);
        }
        assert!(state.at < 3);
    }

    #[test]
    fn a_hovered_ticker_holds_still() {
        let mut state = rolling();
        state.over = true;
        let dwell = Duration::from_secs(1);
        let mut now = Instant::now();
        for _ in 0..30 {
            now += Duration::from_millis(200);
            state.step(now, dwell);
        }
        assert_eq!(state.offset, 0.0);
    }

    #[test]
    fn a_list_that_fits_never_moves() {
        let mut state = State { tops: vec![0.0, 50.0], period: 100.0, moving: false, ..State::default() };
        assert_eq!(state.step(Instant::now(), Duration::from_secs(1)), None);
    }
}
