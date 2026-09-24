use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Point, Rectangle, Renderer, Size, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Target,
    Keys,
    Chain,
    Play,
    Star,
    Clock,
    Heart,
    Trophy,
    Chart,
    News,
    Send,
    Gear,
    Calendar,
    Flame,
    Swords,
    External,
    Compare,
    Up,
    Down,
    Film,
    Flag,
}

pub struct Glyph {
    pub icon: Icon,
    pub alpha: f32,
    pub colour: Color,
}

pub fn glyph<'a, Message: 'a>(icon: Icon, side: f32, colour: Color) -> Element<'a, Message> {
    Canvas::new(Glyph { icon, alpha: crate::ui::fade(), colour }).width(side).height(side).into()
}

fn line(colour: Color, width: f32) -> Stroke<'static> {
    Stroke::default().with_color(colour).with_width(width).with_line_cap(canvas::LineCap::Round).with_line_join(canvas::LineJoin::Round)
}

impl<Message> canvas::Program<Message> for Glyph {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let s = bounds.width.min(bounds.height) / 20.0;
        let at = |x: f32, y: f32| Point::new(x * s, y * s);
        let ink = Color { a: self.colour.a * self.alpha, ..self.colour };
        let pen = line(ink, 1.7 * s);
        let polyline = |points: &[(f32, f32)], close: bool| {
            Path::new(|b| {
                for (index, (x, y)) in points.iter().enumerate() {
                    if index == 0 {
                        b.move_to(at(*x, *y));
                    } else {
                        b.line_to(at(*x, *y));
                    }
                }
                if close {
                    b.close();
                }
            })
        };
        match self.icon {
            Icon::Target => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 8.0 * s), pen);
                frame.stroke(&Path::circle(at(10.0, 10.0), 4.5 * s), pen);
                frame.fill(&Path::circle(at(10.0, 10.0), 1.6 * s), ink);
                frame.stroke(&Path::line(at(10.0, 10.0), at(17.5, 2.5)), pen);
            }
            Icon::Keys => {
                frame.stroke(&Path::rounded_rectangle(at(1.5, 4.5), Size::new(17.0 * s, 11.0 * s), (2.5 * s).into()), pen);
                for (x, y) in [(5.0, 8.0), (8.3, 8.0), (11.7, 8.0), (15.0, 8.0)] {
                    frame.fill(&Path::circle(at(x, y), 1.0 * s), ink);
                }
                frame.stroke(&Path::line(at(6.0, 12.0), at(14.0, 12.0)), pen);
            }
            Icon::Chain => {
                frame.stroke(&Path::rounded_rectangle(at(2.0, 8.5), Size::new(9.0 * s, 5.5 * s), (2.75 * s).into()), pen);
                frame.stroke(&Path::rounded_rectangle(at(9.0, 6.0), Size::new(9.0 * s, 5.5 * s), (2.75 * s).into()), pen);
            }
            Icon::Play => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 8.0 * s), pen);
                frame.fill(&polyline(&[(8.0, 6.3), (14.0, 10.0), (8.0, 13.7)], true), ink);
            }
            Icon::Star => {
                let star = Path::new(|b| {
                    for point in 0..10 {
                        let reach = if point % 2 == 0 { 8.8 } else { 3.9 };
                        let angle = -std::f32::consts::FRAC_PI_2 + point as f32 * std::f32::consts::PI / 5.0;
                        let p = at(10.0 + reach * angle.cos(), 10.6 + reach * angle.sin());
                        if point == 0 {
                            b.move_to(p);
                        } else {
                            b.line_to(p);
                        }
                    }
                    b.close();
                });
                frame.fill(&star, ink);
            }
            Icon::Clock => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 8.0 * s), pen);
                frame.stroke(&polyline(&[(10.0, 5.2), (10.0, 10.0), (13.5, 12.2)], false), pen);
            }
            Icon::Heart => {
                let heart = Path::new(|b| {
                    b.move_to(at(10.0, 17.5));
                    b.bezier_curve_to(at(3.0, 12.5), at(0.5, 8.5), at(3.0, 4.6));
                    b.bezier_curve_to(at(5.2, 1.6), at(9.0, 2.2), at(10.0, 5.6));
                    b.bezier_curve_to(at(11.0, 2.2), at(14.8, 1.6), at(17.0, 4.6));
                    b.bezier_curve_to(at(19.5, 8.5), at(17.0, 12.5), at(10.0, 17.5));
                    b.close();
                });
                frame.fill(&heart, ink);
            }
            Icon::Trophy => {
                let cup = Path::new(|b| {
                    b.move_to(at(6.0, 3.0));
                    b.line_to(at(14.0, 3.0));
                    b.line_to(at(14.0, 8.0));
                    b.bezier_curve_to(at(14.0, 10.5), at(12.2, 12.0), at(10.0, 12.0));
                    b.bezier_curve_to(at(7.8, 12.0), at(6.0, 10.5), at(6.0, 8.0));
                    b.close();
                });
                frame.stroke(&cup, pen);
                frame.stroke(&polyline(&[(6.0, 5.0), (3.0, 5.0), (3.0, 6.5), (5.5, 9.0)], false), pen);
                frame.stroke(&polyline(&[(14.0, 5.0), (17.0, 5.0), (17.0, 6.5), (14.5, 9.0)], false), pen);
                frame.stroke(&Path::line(at(10.0, 12.0), at(10.0, 15.0)), pen);
                frame.stroke(&Path::line(at(7.0, 17.5), at(13.0, 17.5)), pen);
            }
            Icon::Chart => {
                frame.stroke(&Path::line(at(3.0, 16.5), at(17.0, 16.5)), pen);
                frame.stroke(&polyline(&[(5.0, 13.0), (8.5, 9.0), (11.5, 11.5), (16.0, 5.0)], false), pen);
            }
            Icon::News => {
                frame.stroke(&Path::rounded_rectangle(at(2.5, 3.5), Size::new(15.0 * s, 13.0 * s), (2.0 * s).into()), pen);
                for (from, to, y) in [(6.0, 14.0, 7.5), (6.0, 14.0, 10.5), (6.0, 11.0, 13.5)] {
                    frame.stroke(&Path::line(at(from, y), at(to, y)), pen);
                }
            }
            Icon::Send => {
                frame.stroke(&polyline(&[(3.0, 10.0), (17.0, 3.5), (12.0, 17.0), (9.5, 11.5)], true), pen);
            }
            Icon::Gear => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 2.8 * s), pen);
                for (a, b) in [((10.0, 2.5), (10.0, 4.7)), ((10.0, 15.3), (10.0, 17.5)), ((17.5, 10.0), (15.3, 10.0)), ((4.7, 10.0), (2.5, 10.0)), ((15.3, 4.7), (13.7, 6.3)), ((6.3, 13.7), (4.7, 15.3)), ((15.3, 15.3), (13.7, 13.7)), ((6.3, 6.3), (4.7, 4.7))] {
                    frame.stroke(&Path::line(at(a.0, a.1), at(b.0, b.1)), pen);
                }
            }
            Icon::Calendar => {
                frame.stroke(&Path::rounded_rectangle(at(2.5, 4.0), Size::new(15.0 * s, 13.5 * s), (2.5 * s).into()), pen);
                frame.stroke(&Path::line(at(2.5, 8.5), at(17.5, 8.5)), pen);
                frame.stroke(&Path::line(at(6.5, 2.0), at(6.5, 6.0)), pen);
                frame.stroke(&Path::line(at(13.5, 2.0), at(13.5, 6.0)), pen);
            }
            Icon::Flame => {
                let flame = Path::new(|b| {
                    b.move_to(at(10.0, 18.0));
                    b.bezier_curve_to(at(13.6, 18.0), at(16.0, 15.6), at(16.0, 12.4));
                    b.bezier_curve_to(at(16.0, 9.2), at(13.6, 7.4), at(12.6, 4.8));
                    b.bezier_curve_to(at(12.2, 6.6), at(11.2, 7.6), at(10.0, 8.2));
                    b.bezier_curve_to(at(10.2, 5.6), at(9.2, 3.4), at(7.0, 2.0));
                    b.bezier_curve_to(at(7.4, 5.0), at(5.4, 6.6), at(4.4, 8.4));
                    b.bezier_curve_to(at(3.6, 10.0), at(4.0, 11.2), at(4.0, 12.4));
                    b.bezier_curve_to(at(4.0, 15.6), at(6.4, 18.0), at(10.0, 18.0));
                    b.close();
                });
                frame.stroke(&flame, pen);
            }
            Icon::Swords => {
                for (a, b) in [((3.0, 3.0), (11.0, 11.0)), ((17.0, 3.0), (9.0, 11.0)), ((3.0, 17.0), (6.0, 14.0)), ((17.0, 17.0), (14.0, 14.0)), ((5.5, 12.5), (7.5, 14.5)), ((14.5, 12.5), (12.5, 14.5))] {
                    frame.stroke(&Path::line(at(a.0, a.1), at(b.0, b.1)), pen);
                }
            }
            Icon::External => {
                frame.stroke(&polyline(&[(11.0, 3.0), (17.0, 3.0), (17.0, 9.0)], false), pen);
                frame.stroke(&Path::line(at(17.0, 3.0), at(9.0, 11.0)), pen);
                frame.stroke(&polyline(&[(14.0, 11.5), (14.0, 16.0), (13.0, 17.5), (4.5, 17.5), (3.0, 16.0), (3.0, 7.5), (4.5, 6.0), (9.0, 6.0)], false), pen);
            }
            Icon::Compare => {
                frame.stroke(&Path::line(at(6.0, 3.0), at(6.0, 17.0)), pen);
                frame.stroke(&Path::line(at(14.0, 3.0), at(14.0, 17.0)), pen);
                frame.stroke(&polyline(&[(3.0, 7.0), (6.0, 3.0), (9.0, 7.0)], false), pen);
                frame.stroke(&polyline(&[(11.0, 13.0), (14.0, 17.0), (17.0, 13.0)], false), pen);
            }
            Icon::Up => {
                frame.stroke(&Path::line(at(10.0, 16.0), at(10.0, 4.0)), pen);
                frame.stroke(&polyline(&[(5.0, 9.0), (10.0, 4.0), (15.0, 9.0)], false), pen);
            }
            Icon::Down => {
                frame.stroke(&Path::line(at(10.0, 4.0), at(10.0, 16.0)), pen);
                frame.stroke(&polyline(&[(5.0, 11.0), (10.0, 16.0), (15.0, 11.0)], false), pen);
            }
            Icon::Film => {
                frame.stroke(&Path::rounded_rectangle(at(2.5, 4.0), Size::new(15.0 * s, 12.0 * s), (2.0 * s).into()), pen);
                for (a, b) in [((6.5, 4.0), (6.5, 16.0)), ((13.5, 4.0), (13.5, 16.0)), ((2.5, 8.0), (6.5, 8.0)), ((2.5, 12.0), (6.5, 12.0)), ((13.5, 8.0), (17.5, 8.0)), ((13.5, 12.0), (17.5, 12.0))] {
                    frame.stroke(&Path::line(at(a.0, a.1), at(b.0, b.1)), pen);
                }
            }
            Icon::Flag => {
                frame.stroke(&Path::line(at(4.0, 2.5), at(4.0, 17.5)), pen);
                frame.stroke(&polyline(&[(4.0, 3.5), (15.5, 3.5), (13.0, 7.0), (15.5, 10.5), (4.0, 10.5)], false), pen);
            }
        }
        vec![frame.into_geometry()]
    }
}
