use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{renderer, Clipboard, Shell};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Rectangle, Shadow, Size, Theme, Vector};

type Renderer = iced::Renderer;

pub struct Look {
    pub fill: Color,
    pub line: Color,
    pub veil: Color,
    pub radius: f32,
}

pub struct Unfold<'a, Message> {
    children: Vec<Element<'a, Message>>,
    from: Option<Rectangle>,
    k: f32,
    wide: f32,
    room: Padding,
    look: Look,
    on_close: Message,
    fit: bool,
}

pub fn unfold<'a, Message: Clone + 'a>(after: Element<'a, Message>, before: Option<Element<'a, Message>>, from: Option<Rectangle>, k: f32, on_close: Message) -> Unfold<'a, Message> {
    let mut children = vec![after];
    children.extend(before);
    Unfold {
        children,
        from,
        k: k.clamp(0.0, 1.0),
        wide: f32::INFINITY,
        room: Padding::ZERO,
        look: Look { fill: Color::BLACK, line: Color::TRANSPARENT, veil: Color::TRANSPARENT, radius: 16.0 },
        on_close,
        fit: false,
    }
}

impl<'a, Message: Clone + 'a> Unfold<'a, Message> {
    pub fn wide(mut self, wide: f32) -> Self {
        self.wide = wide;
        self
    }

    pub fn room(mut self, room: Padding) -> Self {
        self.room = room;
        self
    }

    pub fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }

    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }
}

fn mix(a: f32, b: f32, k: f32) -> f32 {
    a + (b - a) * k
}

pub fn between(from: Rectangle, to: Rectangle, k: f32) -> Rectangle {
    Rectangle { x: mix(from.x, to.x, k), y: mix(from.y, to.y, k), width: mix(from.width, to.width, k), height: mix(from.height, to.height, k) }
}

pub fn resting(card: Rectangle) -> Rectangle {
    let scale = 0.96;
    let width = card.width * scale;
    let height = card.height * scale;
    Rectangle { x: card.x + (card.width - width) / 2.0, y: card.y + (card.height - height) / 2.0 + 18.0, width, height }
}

impl<Message> Unfold<'_, Message> {
    fn card(&self, bounds: Rectangle) -> Rectangle {
        let room = self.room;
        let across = (bounds.width - room.left - room.right).max(0.0);
        let width = across.min(self.wide);
        Rectangle {
            x: bounds.x + room.left + (across - width) / 2.0,
            y: bounds.y + room.top,
            width,
            height: (bounds.height - room.top - room.bottom).max(0.0),
        }
    }

    fn start(&self, card: Rectangle) -> Rectangle {
        self.from.filter(|from| from.width > 1.0 && from.height > 1.0).unwrap_or_else(|| resting(card))
    }

    fn settled(&self) -> bool {
        self.k >= 0.999
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Unfold<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let area = limits.max();
        let card = self.card(Rectangle::new(iced::Point::ORIGIN, area));
        let start = self.start(card);
        let mut nodes = Vec::with_capacity(self.children.len());
        for (at, (child, state)) in self.children.iter_mut().zip(tree.children.iter_mut()).enumerate() {
            let size = if at == 0 { card.size() } else { start.size() };
            let node = child.as_widget_mut().layout(state, renderer, &layout::Limits::new(Size::ZERO, size));
            let place = if at == 0 && self.fit {
                let high = node.size().height.min(card.height);
                iced::Point::new(card.x, card.y + (card.height - high).max(0.0) * 0.3)
            } else {
                card.position()
            };
            nodes.push(if at == 0 { node.move_to(place) } else { node });
        }
        Node::with_children(area, nodes)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        if let (Some(child), Some(state), Some(place)) = (self.children.first_mut(), tree.children.first_mut(), layout.children().next()) {
            child.as_widget_mut().operate(state, place, renderer, operation);
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
        let settled = self.settled();
        let Some(place) = layout.children().next() else {
            return;
        };
        let card = place.bounds();
        let passing = matches!(event, iced::Event::Window(_)) || settled;
        if passing {
            let inside = if settled { cursor } else { mouse::Cursor::Unavailable };
            if let (Some(child), Some(state)) = (self.children.first_mut(), tree.children.first_mut()) {
                child.as_widget_mut().update(state, event, place, inside, renderer, clipboard, shell, viewport);
            }
        }
        if let iced::Event::Mouse(mouse_event) = event {
            if shell.is_event_captured() {
                return;
            }
            let shown = between(self.start(card), card, self.k);
            if let mouse::Event::ButtonPressed(mouse::Button::Left) = mouse_event {
                if cursor.position().is_some_and(|at| bounds.contains(at) && !shown.contains(at)) {
                    shell.publish(self.on_close.clone());
                }
            }
            if cursor.position().is_some_and(|at| bounds.contains(at)) || matches!(mouse_event, mouse::Event::WheelScrolled { .. }) {
                shell.capture_event();
            }
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let inner = match (self.settled(), self.children.first(), tree.children.first(), layout.children().next()) {
            (true, Some(child), Some(state), Some(place)) => child.as_widget().mouse_interaction(state, place, cursor, viewport, renderer),
            _ => mouse::Interaction::None,
        };
        inner.max(mouse::Interaction::Idle)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let mut places = layout.children();
        let Some(place) = places.next() else {
            return;
        };
        let card = place.bounds();
        let start = self.start(card);
        let k = self.k;
        let shown = between(start, card, k);
        let radius = mix(crate::theme::CARD_RADIUS, self.look.radius, k);
        let veil = Color { a: self.look.veil.a * k, ..self.look.veil };
        renderer.fill_quad(renderer::Quad { bounds, ..renderer::Quad::default() }, Background::Color(veil));
        renderer.fill_quad(
            renderer::Quad {
                bounds: shown,
                border: Border { color: Color { a: self.look.line.a * k.max(0.4), ..self.look.line }, width: 1.0, radius: radius.into() },
                shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.55 * k), offset: Vector::new(0.0, 24.0 * k), blur_radius: 60.0 * k },
                snap: true,
            },
            Background::Color(self.look.fill),
        );
        let inside = if self.settled() { cursor } else { mouse::Cursor::Unavailable };
        renderer.with_layer(shown, |renderer| {
            if let (Some(before), Some(state), Some(spot)) = (self.children.get(1), tree.children.get(1), places.next()) {
                let by = Vector::new(shown.x - spot.bounds().x, shown.y - spot.bounds().y);
                let seen = Rectangle { x: shown.x - by.x, y: shown.y - by.y, ..shown };
                renderer.with_translation(by, |renderer| {
                    before.as_widget().draw(state, renderer, theme, style, spot, mouse::Cursor::Unavailable, &seen);
                });
            }
            if let (Some(after), Some(state)) = (self.children.first(), tree.children.first()) {
                let by = Vector::new(shown.x - card.x, shown.y - card.y);
                let seen = Rectangle { x: card.x, y: card.y, width: shown.width, height: shown.height };
                renderer.with_translation(by, |renderer| {
                    after.as_widget().draw(state, renderer, theme, style, place, inside, &seen);
                });
            }
        });
    }
}

impl<'a, Message: Clone + 'a> From<Unfold<'a, Message>> for Element<'a, Message> {
    fn from(unfold: Unfold<'a, Message>) -> Element<'a, Message> {
        Element::new(unfold)
    }
}

pub struct Shield<'a, Message> {
    content: Element<'a, Message>,
}

pub fn shield<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Shield<'a, Message> {
    Shield { content: content.into() }
}

impl<Message> Widget<Message, Theme, Renderer> for Shield<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
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
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
        if let iced::Event::Mouse(mouse_event) = event {
            let over = cursor.position().is_some_and(|at| layout.bounds().contains(at));
            if !shell.is_event_captured() && (over || matches!(mouse_event, mouse::Event::WheelScrolled { .. })) {
                shell.capture_event();
            }
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer).max(mouse::Interaction::Idle)
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
        self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Shield<'a, Message>> for Element<'a, Message> {
    fn from(shield: Shield<'a, Message>) -> Element<'a, Message> {
        Element::new(shield)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stage_grows_out_of_the_panel_and_lands_on_the_card() {
        let from = Rectangle::new(iced::Point::new(40.0, 300.0), Size::new(445.0, 336.0));
        let card = Rectangle::new(iced::Point::new(200.0, 60.0), Size::new(900.0, 700.0));
        assert_eq!(between(from, card, 0.0), from);
        assert_eq!(between(from, card, 1.0), card);
        let halfway = between(from, card, 0.5);
        assert!(halfway.width > from.width && halfway.width < card.width);
        assert!(halfway.y < from.y && halfway.y > card.y);
    }

    #[test]
    fn without_a_panel_the_stage_rises_from_a_little_below() {
        let card = Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(1000.0, 800.0));
        let start = resting(card);
        assert!(start.width < card.width && start.y > card.y);
        assert!((start.center_x() - card.center_x()).abs() < 0.01);
    }
}
