use std::time::Instant;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::Operation;
use iced::advanced::{overlay, renderer, Clipboard, Renderer as _, Shell, Widget};
use iced::{mouse, touch, window, Background, Border, Color, Element, Event, Length, Padding, Rectangle, Renderer, Shadow, Size, Theme, Vector};

pub use iced::widget::button::{Status, Style};

use crate::{motion, theme, ui};

const SINK: f32 = 0.5;
const PADDING: Padding = Padding { top: 5.0, bottom: 5.0, right: 10.0, left: 10.0 };

pub fn button<'a, Message>(content: impl Into<Element<'a, Message>>) -> Button<'a, Message> {
    Button::new(content)
}

pub struct Button<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    width: Length,
    height: Length,
    padding: Padding,
    clip: bool,
    style: Box<dyn Fn(&Theme, Status) -> Style + 'a>,
}

impl<'a, Message> Button<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();
        Button {
            content,
            on_press: None,
            width: size.width.fluid(),
            height: size.height.fluid(),
            padding: PADDING,
            clip: false,
            style: Box::new(iced::widget::button::primary),
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press;
        self
    }

    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }
}

#[derive(Debug, Default)]
struct State {
    held: bool,
    status: Option<Status>,
    hover: f32,
    press: f32,
    last: Option<Instant>,
}

fn part(from: f32, to: f32, k: f32) -> f32 {
    from + (to - from) * k
}

fn tint(from: Color, to: Color, k: f32) -> Color {
    if from.a <= 0.0 {
        return Color { a: to.a * k, ..to };
    }
    if to.a <= 0.0 {
        return Color { a: from.a * (1.0 - k), ..from };
    }
    ui::mix(from, to, k)
}

fn blend(from: Style, to: Style, k: f32) -> Style {
    if k >= 1.0 {
        return to;
    }
    let background = match (from.background, to.background) {
        (Some(Background::Color(a)), Some(Background::Color(b))) => Some(Background::Color(tint(a, b, k))),
        (None, Some(Background::Color(b))) => Some(Background::Color(Color { a: b.a * k, ..b })),
        (Some(Background::Color(a)), None) => Some(Background::Color(Color { a: a.a * (1.0 - k), ..a })),
        (None, None) => None,
        (a, b) => if k < 0.5 { a } else { b },
    };
    Style {
        background,
        text_color: tint(from.text_color, to.text_color, k),
        border: Border {
            color: tint(from.border.color, to.border.color, k),
            width: part(from.border.width, to.border.width, k),
            radius: to.border.radius,
        },
        shadow: Shadow {
            color: tint(from.shadow.color, to.shadow.color, k),
            offset: Vector::new(part(from.shadow.offset.x, to.shadow.offset.x, k), part(from.shadow.offset.y, to.shadow.offset.y, k)),
            blur_radius: part(from.shadow.blur_radius, to.shadow.blur_radius, k),
        },
        snap: to.snap,
    }
}

fn sunk(style: Style, speaks: bool) -> Style {
    let ground = theme::GROUND;
    match style.background {
        Some(Background::Color(fill)) if fill.a > 0.0 => {
            let under = fill.a * (1.0 - SINK);
            let alpha = SINK + under;
            let over = |ground: f32, own: f32| (ground * SINK + own * under) / alpha;
            Style { background: Some(Background::Color(Color { r: over(ground.r, fill.r), g: over(ground.g, fill.g), b: over(ground.b, fill.b), a: alpha })), ..style }
        }
        Some(Background::Color(_)) | None if !speaks => Style { background: Some(Background::Color(Color { a: SINK, ..ground })), ..style },
        _ => style,
    }
}

impl<'a, Message> Button<'a, Message> {
    fn look(&self, theme: &Theme, state: &State) -> Style {
        let status = state.status.unwrap_or(if self.on_press.is_some() { Status::Active } else { Status::Disabled });
        if status == Status::Disabled {
            return (self.style)(theme, Status::Disabled);
        }
        let rest = (self.style)(theme, Status::Active);
        if state.hover <= 0.0 && state.press <= 0.0 {
            return rest;
        }
        let lit = (self.style)(theme, Status::Hovered);
        let shown = blend(rest, lit, state.hover);
        if state.press <= 0.0 {
            return shown;
        }
        let down = (self.style)(theme, Status::Pressed);
        let down = if down.background == lit.background { sunk(down, lit.text_color != rest.text_color) } else { down };
        blend(shown, down, state.press)
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Button<'a, Message> {
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
        Size { width: self.width, height: self.height }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits))
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(&mut tree.children[0], layout.children().next().unwrap(), renderer, operation);
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout.children().next().unwrap(), cursor, renderer, clipboard, shell, viewport);
        if shell.is_event_captured() {
            return;
        }
        let state = tree.state.downcast_mut::<State>();
        let over = cursor.is_over(layout.bounds());
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if self.on_press.is_some() && over {
                    state.held = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = &self.on_press {
                    if state.held {
                        state.held = false;
                        if over {
                            shell.publish(on_press.clone());
                        }
                        shell.capture_event();
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => state.held = false,
            _ => {}
        }
        let current = if self.on_press.is_none() {
            Status::Disabled
        } else if over {
            if state.held { Status::Pressed } else { Status::Hovered }
        } else {
            Status::Active
        };
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let fresh = state.status.is_none();
            state.status = Some(current);
            let (hover, press) = match current {
                Status::Hovered => (1.0, 0.0),
                Status::Pressed => (1.0, 1.0),
                Status::Active | Status::Disabled => (0.0, 0.0),
            };
            if press > 0.5 {
                state.press = 1.0;
            }
            let settled = (state.hover - hover).abs() < motion::SETTLED && (state.press - press).abs() < motion::SETTLED;
            if fresh || settled || current == Status::Disabled {
                state.hover = hover;
                state.press = press;
                state.last = None;
            } else {
                let dt = ui::elapsed(&mut state.last, *now);
                state.hover = ui::toward(state.hover, hover, motion::RESPONSE, dt);
                state.press = ui::toward(state.press, press, motion::RESPONSE, dt);
                shell.request_redraw();
            }
        } else if state.status.is_some_and(|status| status != current) {
            shell.request_redraw();
        }
    }

    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, _: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        let bounds = layout.bounds();
        let style = self.look(theme, tree.state.downcast_ref::<State>());
        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad { bounds, border: style.border, shadow: style.shadow, snap: style.snap },
                style.background.unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }
        let viewport = if self.clip { bounds.intersection(viewport).unwrap_or(*viewport) } else { *viewport };
        self.content.as_widget().draw(&tree.children[0], renderer, theme, &renderer::Style { text_color: style.text_color }, layout.children().next().unwrap(), cursor, &viewport);
    }

    fn mouse_interaction(&self, _: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, _: &Rectangle, _: &Renderer) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) && self.on_press.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout.children().next().unwrap(), renderer, viewport, translation)
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(button: Button<'a, Message>) -> Self {
        Element::new(button)
    }
}

#[cfg(test)]
mod tests {
    use super::{blend, sunk, Style};
    use iced::{Background, Color};

    fn filled(colour: Color) -> Style {
        Style { background: Some(Background::Color(colour)), ..Style::default() }
    }

    fn fill(style: Style) -> Color {
        match style.background {
            Some(Background::Color(colour)) => colour,
            _ => Color::TRANSPARENT,
        }
    }

    #[test]
    fn a_fill_that_comes_from_nothing_keeps_its_own_colour_on_the_way() {
        let half = fill(blend(Style::default(), filled(Color::from_rgba(1.0, 0.5, 0.25, 0.8)), 0.5));
        assert!((half.r - 1.0).abs() < 1e-5 && (half.g - 0.5).abs() < 1e-5, "{half:?}");
        assert!((half.a - 0.4).abs() < 1e-5, "{half:?}");
    }

    #[test]
    fn a_pressed_fill_is_darker_and_denser_than_the_lit_one() {
        let lit = Color::from_rgba(0.17, 0.075, 0.098, 0.66);
        let down = fill(sunk(filled(lit), false));
        assert!(down.r < lit.r && down.a > lit.a, "{down:?}");
        let wash = fill(sunk(filled(Color::from_rgba(1.0, 1.0, 1.0, 0.05)), false));
        assert!(wash.r < 0.15, "a pale wash sinks to a dark pad, got {wash:?}");
    }

    #[test]
    fn a_button_that_speaks_through_its_words_gets_no_pad() {
        assert_eq!(sunk(Style::default(), true).background, None);
        assert!(sunk(Style::default(), false).background.is_some());
    }
}
