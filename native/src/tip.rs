use std::cell::Cell;
use std::time::{Duration, Instant};

use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{layout, overlay, renderer, Clipboard, Layout, Renderer as _, Shell, Widget};
use iced::{mouse, window, Background, Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Shadow, Size, Theme, Vector};

use crate::theme;

pub const DELAY: Duration = Duration::from_millis(350);
pub const FADE: Duration = Duration::from_millis(120);
pub const NEIGHBOUR: Duration = Duration::from_millis(400);

const GAP: f32 = 7.0;
const SLIDE: f32 = 3.0;
const MOST_WIDE: f32 = 340.0;
const WINDOW_EDGE: f32 = 6.0;
const SIDE_ROOM: f32 = 11.0;
const TOP_ROOM: f32 = 7.0;
const WORDS_SIZE: f32 = 12.5;
const UNDER_SIZE: f32 = 11.5;
const KEYS_ROOM: f32 = 16.0;
const LINES_ROOM: f32 = 4.0;
const RADIUS: f32 = 8.0;
const FILL: Color = Color::from_rgb(0.102, 0.063, 0.078);

type Para = <Renderer as text::Renderer>::Paragraph;

thread_local! { static LAST_SHUT: Cell<Option<Instant>> = const { Cell::new(None) }; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,
    Waiting(Instant),
    Open(Instant),
    Shut,
}

fn next(phase: Phase, over: bool, pressed: bool, now: Instant, last_shut: Option<Instant>) -> Phase {
    if !over {
        return Phase::Idle;
    }
    match phase {
        Phase::Idle if pressed => Phase::Shut,
        Phase::Idle => match last_shut.filter(|at| now.saturating_duration_since(*at) < NEIGHBOUR) {
            Some(_) => Phase::Open(now.checked_sub(FADE).unwrap_or(now)),
            None => Phase::Waiting(now),
        },
        Phase::Waiting(_) | Phase::Open(_) if pressed => Phase::Shut,
        Phase::Waiting(at) if now.saturating_duration_since(at) >= DELAY => Phase::Open(now),
        kept => kept,
    }
}

fn shown(phase: Phase, now: Instant) -> f32 {
    let Phase::Open(at) = phase else {
        return 0.0;
    };
    let k = (now.saturating_duration_since(at).as_secs_f32() / FADE.as_secs_f32()).clamp(0.0, 1.0);
    1.0 - (1.0 - k) * (1.0 - k)
}

fn placed(anchor: Rectangle, size: Size, room: Size, side: Side) -> Point {
    let above = anchor.y - GAP - size.height;
    let below = anchor.y + anchor.height + GAP;
    let fits_above = above >= WINDOW_EDGE;
    let fits_below = below + size.height <= room.height - WINDOW_EDGE;
    let y = match side {
        Side::Top if fits_above || !fits_below => above,
        Side::Bottom if fits_below || !fits_above => below,
        Side::Top => below,
        Side::Bottom => above,
    };
    let x = anchor.x + (anchor.width - size.width) / 2.0;
    let most_x = (room.width - WINDOW_EDGE - size.width).max(WINDOW_EDGE);
    Point::new(x.clamp(WINDOW_EDGE, most_x), y.clamp(WINDOW_EDGE, (room.height - WINDOW_EDGE - size.height).max(WINDOW_EDGE)))
}

fn line(words: &str, font: Font, size: f32, wide: f32) -> Para {
    Para::with_text(text::Text {
        content: words,
        bounds: Size::new(wide, f32::INFINITY),
        size: Pixels(size),
        line_height: text::LineHeight::Relative(1.35),
        font,
        align_x: text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::WordOrGlyph,
    })
}

struct Laid {
    key: (String, Option<String>, Option<String>),
    words: Para,
    under: Option<Para>,
    keys: Option<Para>,
}

impl Laid {
    fn size(&self) -> Size {
        let keys = self.keys.as_ref().map_or(0.0, |keys| keys.min_bounds().width + KEYS_ROOM);
        let top = self.words.min_bounds();
        let under = self.under.as_ref().map(|under| under.min_bounds());
        let wide = (top.width + keys).max(under.map_or(0.0, |under| under.width));
        let high = top.height + under.map_or(0.0, |under| under.height + LINES_ROOM);
        Size::new((wide + SIDE_ROOM * 2.0).ceil(), (high + TOP_ROOM * 2.0).ceil())
    }
}

#[derive(Default)]
struct State {
    phase: Phase,
    laid: Option<Laid>,
}

pub struct Tip<'a, Message> {
    content: Element<'a, Message>,
    words: String,
    under: Option<String>,
    keys: Option<String>,
    side: Side,
    pinned: bool,
}

impl<'a, Message: 'a> Tip<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, words: impl Into<String>) -> Self {
        Tip { content: content.into(), words: words.into(), under: None, keys: None, side: Side::Top, pinned: false }
    }

    pub fn under(mut self, words: impl Into<String>) -> Self {
        self.under = Some(words.into());
        self
    }

    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Tip<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let phase = if self.pinned { Phase::Open(Instant::now().checked_sub(FADE).unwrap_or_else(Instant::now)) } else { Phase::Idle };
        tree::State::new(State { phase, laid: None })
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

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        if !self.pinned && matches!(event, Event::Mouse(_) | Event::Window(window::Event::RedrawRequested(_))) {
            let state = tree.state.downcast_mut::<State>();
            let now = Instant::now();
            let pressed = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)));
            let was = state.phase;
            state.phase = next(was, cursor.is_over(layout.bounds()), pressed, now, LAST_SHUT.with(Cell::get));
            let (opened, closed) = (matches!(state.phase, Phase::Open(_)), matches!(was, Phase::Open(_)));
            if closed && !opened {
                LAST_SHUT.with(|last| last.set(Some(now)));
            }
            if opened != closed {
                shell.invalidate_layout();
                shell.request_redraw();
            }
            match state.phase {
                Phase::Waiting(at) => shell.request_redraw_at(at + DELAY),
                Phase::Open(at) if now.saturating_duration_since(at) < FADE => shell.request_redraw(),
                _ => {}
            }
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree { state, children, .. } = tree;
        let inner = self.content.as_widget_mut().overlay(&mut children[0], layout, renderer, viewport, translation);
        let state = state.downcast_mut::<State>();
        let k = shown(state.phase, Instant::now());
        let tip = matches!(state.phase, Phase::Open(_)).then(|| {
            let key = (self.words.clone(), self.under.clone(), self.keys.clone());
            if state.laid.as_ref().is_none_or(|laid| laid.key != key) {
                let wide = MOST_WIDE - SIDE_ROOM * 2.0;
                state.laid = Some(Laid {
                    words: line(&self.words, if self.under.is_some() { theme::SANS_SEMI } else { theme::SANS }, WORDS_SIZE, wide),
                    under: self.under.as_deref().map(|under| line(under, theme::SANS, UNDER_SIZE, wide)),
                    keys: self.keys.as_deref().map(|keys| line(keys, theme::SANS, UNDER_SIZE, wide)),
                    key,
                });
            }
            overlay::Element::new(Box::new(Told { laid: state.laid.as_ref().expect("just laid"), anchor: layout.bounds() + translation, side: self.side, k }))
        });
        match (inner, tip) {
            (None, None) => None,
            (inner, tip) => Some(overlay::Group::with_children(inner.into_iter().chain(tip).collect()).overlay()),
        }
    }
}

struct Told<'b> {
    laid: &'b Laid,
    anchor: Rectangle,
    side: Side,
    k: f32,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Told<'_> {
    fn layout(&mut self, _renderer: &Renderer, bounds: Size) -> layout::Node {
        let size = self.laid.size();
        layout::Node::new(size).move_to(placed(self.anchor, size, bounds, self.side))
    }

    fn draw(&self, renderer: &mut Renderer, _theme: &Theme, _style: &renderer::Style, layout: Layout<'_>, _cursor: mouse::Cursor) {
        let k = self.k;
        let towards = if layout.bounds().y < self.anchor.y { 1.0 } else { -1.0 };
        let bounds = layout.bounds() + Vector::new(0.0, towards * SLIDE * (1.0 - k));
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border { color: theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.07 * k)), width: 1.0, radius: RADIUS.into() },
                shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.5 * k), offset: Vector::new(0.0, 6.0), blur_radius: 18.0 },
                snap: true,
            },
            Background::Color(Color { a: k, ..FILL }),
        );
        let faded = |colour: Color| Color { a: colour.a * k, ..colour };
        let at = Point::new(bounds.x + SIDE_ROOM, bounds.y + TOP_ROOM);
        renderer.fill_paragraph(&self.laid.words, at, faded(theme::INK), bounds);
        if let Some(keys) = &self.laid.keys {
            let right = bounds.x + bounds.width - SIDE_ROOM - keys.min_bounds().width;
            renderer.fill_paragraph(keys, Point::new(right, at.y + 1.0), faded(theme::FAINT), bounds);
        }
        if let Some(under) = &self.laid.under {
            renderer.fill_paragraph(under, Point::new(at.x, at.y + self.laid.words.min_bounds().height + LINES_ROOM), faded(theme::MUTED), bounds);
        }
    }
}

impl<'a, Message: 'a> From<Tip<'a, Message>> for Element<'a, Message> {
    fn from(tip: Tip<'a, Message>) -> Self {
        Element::new(tip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tip_waits_opens_shuts_on_a_press_and_a_neighbour_opens_at_once() {
        let at = Instant::now();
        let hover = next(Phase::Idle, true, false, at, None);
        assert_eq!(hover, Phase::Waiting(at));
        assert_eq!(next(hover, true, false, at + DELAY / 2, None), hover);
        let open = next(hover, true, false, at + DELAY, None);
        assert_eq!(open, Phase::Open(at + DELAY));
        assert_eq!(next(open, true, false, at + DELAY * 2, None), open);
        assert_eq!(next(open, false, false, at + DELAY * 2, None), Phase::Idle);
        assert_eq!(next(hover, false, false, at, None), Phase::Idle);
        let shut = next(open, true, true, at + DELAY * 2, None);
        assert_eq!(shut, Phase::Shut, "a press takes the tip away");
        assert_eq!(next(shut, true, false, at + DELAY * 3, None), Phase::Shut, "and it stays away until the pointer leaves");
        assert_eq!(next(Phase::Idle, true, true, at, None), Phase::Shut);
        let later = at + Duration::from_secs(5);
        let neighbour = next(Phase::Idle, true, false, later, Some(later - NEIGHBOUR / 2));
        assert!(matches!(neighbour, Phase::Open(_)));
        assert_eq!(shown(neighbour, later), 1.0, "without fading in again");
        assert_eq!(next(Phase::Idle, true, false, later, Some(later - NEIGHBOUR)), Phase::Waiting(later));
    }

    #[test]
    fn an_open_tip_is_drawn_over_the_page_beside_what_it_explains() {
        use iced::widget::{column, container, row, text};
        let page: Element<'_, ()> = container(
            column![
                row![
                    Tip::new(container(text("Открыть")).padding(8), "Откроет карту на сайте osu!").pinned(),
                    Tip::new(container(text("Обновить")).padding(8), "Обновить").under("Обновлено сегодня в 14:02").side(Side::Bottom).pinned(),
                    Tip::new(container(text("Команды")).padding(8), "Команды пула").keys("Ctrl+K").side(Side::Bottom).pinned(),
                ]
                .spacing(150),
                Tip::new(container(text("Уровень")).padding(8), "Образцовая карта учит модель слоту с полным весом, подходящая учит как обычно, а пограничная даёт мягкую цель со вторым слотом.").side(Side::Bottom).pinned(),
            ]
            .spacing(30),
        )
        .padding(iced::Padding { top: 60.0, right: 20.0, bottom: 20.0, left: 60.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style { background: Some(Background::Color(Color::from_rgb(0.2, 0.05, 0.08))), ..Default::default() })
        .into();
        let mut ui = iced_test::Simulator::with_size(crate::settings(), Size::new(700.0, 260.0), page);
        let stem = std::env::temp_dir().join(format!("dossier-tip-{}", std::process::id()));
        ui.snapshot(&theme::theme()).unwrap().matches_image(&stem).unwrap();
        let path = crate::gallery::written_as(&stem);
        let picture = image::open(&path).unwrap().to_rgba8();
        let scale = picture.width() / 700;
        let filled = (0..picture.width()).filter(|x| picture.get_pixel(*x, 30 * scale)[0] < 40).count();
        assert!(filled > (60 * scale) as usize, "the tip over the first button: {filled}");
    }

    #[test]
    fn a_tip_fades_in_over_its_time() {
        let at = Instant::now();
        assert_eq!(shown(Phase::Idle, at), 0.0);
        assert_eq!(shown(Phase::Waiting(at), at + DELAY), 0.0);
        assert_eq!(shown(Phase::Open(at), at), 0.0);
        let half = shown(Phase::Open(at), at + FADE / 2);
        assert!(half > 0.5 && half < 1.0, "{half}");
        assert_eq!(shown(Phase::Open(at), at + FADE), 1.0);
    }

    #[test]
    fn a_tip_turns_over_and_slides_along_rather_than_leave_the_window() {
        let room = Size::new(400.0, 300.0);
        let size = Size::new(100.0, 30.0);
        let middle = Rectangle::new(Point::new(150.0, 150.0), Size::new(40.0, 20.0));
        assert_eq!(placed(middle, size, room, Side::Top), Point::new(120.0, 113.0));
        assert_eq!(placed(middle, size, room, Side::Bottom), Point::new(120.0, 177.0));
        let high = Rectangle::new(Point::new(150.0, 10.0), Size::new(40.0, 20.0));
        assert_eq!(placed(high, size, room, Side::Top).y, 37.0, "no room above");
        let low = Rectangle::new(Point::new(150.0, 270.0), Size::new(40.0, 20.0));
        assert_eq!(placed(low, size, room, Side::Bottom).y, 233.0, "no room below");
        let left = Rectangle::new(Point::new(0.0, 150.0), Size::new(20.0, 20.0));
        assert_eq!(placed(left, size, room, Side::Top).x, WINDOW_EDGE);
        let right = Rectangle::new(Point::new(380.0, 150.0), Size::new(20.0, 20.0));
        assert_eq!(placed(right, size, room, Side::Top).x, 294.0);
    }
}
