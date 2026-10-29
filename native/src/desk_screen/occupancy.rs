use iced::widget::{canvas, stack};
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use super::{Message, State, PICKED};

pub fn colour(player: i64) -> Color {
    let hue = (player as f64 * 0.618033988749895).rem_euclid(1.0) as f32 * 6.0;
    let x = 1.0 - (hue % 2.0 - 1.0).abs();
    let (r, g, b) = match hue as u8 {
        0 => (1.0, x, 0.0), 1 => (x, 1.0, 0.0), 2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0), 4 => (x, 0.0, 1.0), _ => (1.0, 0.0, x),
    };
    Color::from_rgb(0.35 + r * 0.6, 0.35 + g * 0.6, 0.35 + b * 0.6)
}

pub fn own(state: &State) -> Color {
    state.presence.iter().find(|person| person.mine).map_or(PICKED, |person| colour(person.player))
}

pub fn mark<'a>(state: &State, content: Element<'a, Message>, selected: bool, radius: f32,
    matches: impl Fn(&crate::desk::Presence) -> bool) -> Element<'a, Message> {
    let mut people: Vec<_> = state.presence.iter().filter(|person| !person.mine && matches(person)).collect();
    people.sort_by_key(|person| person.player);
    let colours: Vec<_> = people.iter().map(|person| colour(person.player)).collect();
    if colours.is_empty() && !selected { return content; }
    let names = people.iter().map(|person| person.name.as_str()).collect::<Vec<_>>().join(", ");
    let drawing = canvas(Mark { colours, border: selected.then(|| own(state)), radius, phase: state.busy_frame as f32 * 0.6 })
        .width(Length::Fill).height(Length::Fill);
    let marked = stack![content, drawing];
    if names.is_empty() { marked.into() } else {
        iced::widget::tooltip(marked, iced::widget::text(names).font(crate::theme::SANS).size(13), iced::widget::tooltip::Position::Top).into()
    }
}

struct Mark { colours: Vec<Color>, border: Option<Color>, radius: f32, phase: f32 }

impl canvas::Program<Message> for Mark {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        crate::ui::stroke_dashes(&mut frame, bounds.size(), self.radius, &self.colours, self.phase);
        if let Some(colour) = self.border {
            let inset = if self.colours.is_empty() { 1.0 } else { 3.0 };
            frame.stroke(&Path::rounded_rectangle(Point::new(inset, inset), Size::new((bounds.width - 2.0 * inset).max(0.0), (bounds.height - 2.0 * inset).max(0.0)), (self.radius - inset).max(0.0).into()), Stroke::default().with_width(1.5).with_color(colour));
        }
        vec![frame.into_geometry()]
    }
}
