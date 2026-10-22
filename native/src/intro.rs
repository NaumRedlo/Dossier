use std::time::{Duration, Instant};

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, float, mouse_area, opaque, stack, Space};
use iced::{mouse, Background, Color, Element, Length, Point, Rectangle, Renderer, Theme, Vector};

use crate::{theme, ui};

pub const DARK: f32 = 0.25;
pub const LOGO_IN: f32 = 0.8;
pub const SPIN_AFTER: f32 = 0.7;
pub const SPIN_IN: f32 = 0.5;
pub const LEAST: Duration = Duration::from_millis(1800);
pub const MOST: Duration = Duration::from_secs(10);
pub const OUT: f32 = 0.7;
pub const LOGO: f32 = 2.2;
pub const RISE: f32 = 10.0;
pub const SPINNER: f32 = 44.0;
pub const SPINNER_GAP: f32 = 48.0;
const TRAIL: usize = 28;

#[derive(Debug, Clone, Copy)]
pub struct Intro {
    pub born: Instant,
    pub now: Instant,
    pub leaving: Option<Instant>,
}

fn eased(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    1.0 - (1.0 - k).powi(3)
}

impl Intro {
    pub fn new(now: Instant) -> Intro {
        Intro { born: now, now, leaving: None }
    }

    fn age(&self) -> f32 {
        self.now.saturating_duration_since(self.born).as_secs_f32()
    }

    fn out(&self) -> f32 {
        self.leaving.map_or(0.0, |at| (self.now.saturating_duration_since(at).as_secs_f32() / OUT).clamp(0.0, 1.0))
    }

    pub fn logo(&self) -> f32 {
        eased((self.age() - DARK) / LOGO_IN) * (1.0 - eased(self.out() * 1.6))
    }

    pub fn spinner(&self) -> f32 {
        eased((self.age() - DARK - SPIN_AFTER) / SPIN_IN) * (1.0 - eased(self.out() * 2.0))
    }

    pub fn veil(&self) -> f32 {
        1.0 - eased(((self.out() - 0.3) / 0.7).max(0.0))
    }

    pub fn may_leave(&self, ready: bool) -> bool {
        let age = self.now.saturating_duration_since(self.born);
        self.leaving.is_none() && ((ready && age >= LEAST) || age >= MOST)
    }

    pub fn done(&self) -> bool {
        self.out() >= 1.0
    }

    pub fn view<'a, Message: Clone + 'a>(&self, swallow: Message) -> Element<'a, Message> {
        let veil = self.veil();
        let logo = self.logo();
        let spin = self.spinner();
        let ground = container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: veil, ..theme::GROUND })),
            ..container::Style::default()
        });
        let rise = (1.0 - logo) * RISE;
        let mark = float(ui::fading(logo, || ui::brand_scaled(LOGO))).translate(move |_, _| Vector::new(0.0, rise));
        let turning = Canvas::new(Spinner { seconds: self.age(), alpha: spin }).width(SPINNER).height(SPINNER);
        let middle = column![mark, Space::new().height(SPINNER_GAP), turning].align_x(iced::Center);
        let middle = container(middle).width(Length::Fill).height(Length::Fill).center(Length::Fill);
        let layers = stack![ground, middle].width(Length::Fill).height(Length::Fill);
        if veil > 0.5 {
            opaque(mouse_area(layers).on_press(swallow).interaction(mouse::Interaction::Idle))
        } else {
            layers.into()
        }
    }
}

pub struct Spinner {
    pub seconds: f32,
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        vec![arc(renderer, bounds, self.seconds, self.alpha, 3.0)]
    }
}

pub fn arc(renderer: &Renderer, bounds: Rectangle, seconds: f32, alpha: f32, width: f32) -> Geometry {
    let mut frame = Frame::new(renderer, bounds.size());
    if alpha <= 0.001 {
        return frame.into_geometry();
    }
    let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
    let radius = bounds.width.min(bounds.height) / 2.0 - width;
    let tau = std::f32::consts::TAU;
    let turn = seconds * tau * 0.85;
    let sweep = tau * (0.22 + 0.42 * (0.5 - 0.5 * (seconds * std::f32::consts::PI * 0.9).cos()));
    for piece in 0..TRAIL {
        let from = piece as f32 / TRAIL as f32;
        let to = (piece + 1) as f32 / TRAIL as f32;
        let start = turn - sweep * (1.0 - from);
        let end = turn - sweep * (1.0 - to);
        let path = Path::new(|path| {
            path.arc(canvas::path::Arc { center: centre, radius, start_angle: iced::Radians(start), end_angle: iced::Radians(end) });
        });
        let shade = to.powf(1.6);
        let cap = if piece + 1 == TRAIL { canvas::LineCap::Round } else { canvas::LineCap::Butt };
        frame.stroke(&path, Stroke::default().with_width(width).with_line_cap(cap).with_color(Color { a: shade * alpha, ..theme::ACCENT }));
    }
    frame.into_geometry()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_intro_starts_dark_shows_the_logo_then_the_circle_and_waits_for_the_library() {
        let born = Instant::now();
        let at = |seconds: f32| Intro { born, now: born + Duration::from_secs_f32(seconds), leaving: None };
        assert_eq!(at(0.0).logo(), 0.0);
        assert_eq!(at(0.0).veil(), 1.0);
        assert!(at(0.6).logo() > 0.0 && at(0.6).spinner() == 0.0, "the logo comes before the circle");
        assert!(at(1.6).spinner() > 0.9);
        assert!(!at(1.0).may_leave(true), "too early even when ready");
        assert!(!at(3.0).may_leave(false), "waits for the library");
        assert!(at(3.0).may_leave(true));
        assert!(at(10.5).may_leave(false), "never stays forever");
        let leaving = |seconds: f32| Intro { born, now: born + Duration::from_secs_f32(3.0 + seconds), leaving: Some(born + Duration::from_secs(3)) };
        assert!(leaving(0.2).veil() > 0.99, "the logo leaves before the dark lifts");
        assert!(leaving(0.4).logo() < 0.1);
        assert!(!leaving(0.5).done());
        assert!(leaving(0.7).done() && leaving(0.7).veil() == 0.0);
    }
}
