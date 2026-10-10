use crate::draw::{line_height, text_width, Canvas, Face, Rgba};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cross {
    Start,
    Centre,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    pub side: f32,
    pub top: f32,
    pub bottom: f32,
    pub radius: f32,
    pub fill: Rgba,
    pub edge: Option<Rgba>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Row {
        gap: f32,
        cross: Cross,
        wide: Option<f32>,
        items: Vec<Node>,
    },
    Col {
        gap: f32,
        cross: Cross,
        wide: Option<f32>,
        items: Vec<Node>,
    },
    Text {
        text: String,
        face: Face,
        size: f32,
        colour: Rgba,
    },
    Plate {
        shell: Shell,
        wide: Option<f32>,
        high: Option<f32>,
        child: Box<Node>,
    },
    Morph {
        shell: Shell,
        from: Box<Node>,
        to: Box<Node>,
        shown: f32,
    },
    Dot {
        side: f32,
        colour: Rgba,
        glow: f32,
    },
    Logo {
        side: f32,
        colour: Rgba,
    },
    Tick {
        side: f32,
        colour: Rgba,
    },
    Badge {
        side: f32,
        radius: f32,
        colour: Rgba,
        sign: String,
    },
    Disc {
        side: f32,
        colour: Rgba,
        sign: String,
        ring: Option<Rgba>,
    },
    Star {
        side: f32,
        colour: Rgba,
    },
    Track {
        share: f32,
        mark: Option<f32>,
        fill: Rgba,
    },
    Cover {
        wide: f32,
        high: f32,
        radius: f32,
        from: Rgba,
        to: Rgba,
    },
    Rule,
    Grow,
}

pub fn row(gap: f32, items: Vec<Node>) -> Node {
    Node::Row {
        gap,
        cross: Cross::Centre,
        wide: None,
        items,
    }
}

pub fn col(gap: f32, items: Vec<Node>) -> Node {
    Node::Col {
        gap,
        cross: Cross::Start,
        wide: None,
        items,
    }
}

pub fn text(text: impl Into<String>, face: Face, size: f32, colour: Rgba) -> Node {
    Node::Text {
        text: text.into(),
        face,
        size,
        colour,
    }
}

impl Node {
    pub fn wide(mut self, width: f32) -> Node {
        match &mut self {
            Node::Row { wide, .. } | Node::Col { wide, .. } | Node::Plate { wide, .. } => {
                *wide = Some(width)
            }
            _ => {}
        }
        self
    }

    pub fn crossed(mut self, side: Cross) -> Node {
        if let Node::Row { cross, .. } | Node::Col { cross, .. } = &mut self {
            *cross = side;
        }
        self
    }

    fn stretches(&self) -> bool {
        match self {
            Node::Track { .. } | Node::Rule => true,
            Node::Row { items, wide, .. } => {
                wide.is_none() && items.iter().any(|item| matches!(item, Node::Grow))
            }
            Node::Col { items, wide, .. } => wide.is_none() && items.iter().any(Node::stretches),
            _ => false,
        }
    }

    pub fn measure(&self) -> (f32, f32) {
        match self {
            Node::Row {
                gap, wide, items, ..
            } => {
                let sizes: Vec<(f32, f32)> = items.iter().map(Node::measure).collect();
                let natural = sizes.iter().map(|size| size.0).sum::<f32>()
                    + gap * items.len().saturating_sub(1) as f32;
                let high = sizes.iter().map(|size| size.1).fold(0.0, f32::max);
                (wide.unwrap_or(natural), high)
            }
            Node::Col {
                gap, wide, items, ..
            } => {
                let sizes: Vec<(f32, f32)> = items.iter().map(Node::measure).collect();
                let natural = sizes.iter().map(|size| size.0).fold(0.0, f32::max);
                let high = sizes.iter().map(|size| size.1).sum::<f32>()
                    + gap * items.len().saturating_sub(1) as f32;
                (wide.unwrap_or(natural), high)
            }
            Node::Text {
                text, face, size, ..
            } => (text_width(text, *face, *size).ceil(), line_height(*size)),
            Node::Plate {
                shell,
                wide,
                high,
                child,
            } => {
                let inside = child.measure();
                (
                    wide.unwrap_or(inside.0 + shell.side * 2.0),
                    high.unwrap_or(inside.1 + shell.top + shell.bottom),
                )
            }
            Node::Morph {
                shell,
                from,
                to,
                shown,
            } => {
                let (a, b) = (from.measure(), to.measure());
                let k = shown.clamp(0.0, 1.0);
                (
                    a.0 + (b.0 - a.0) * k + shell.side * 2.0,
                    a.1 + (b.1 - a.1) * k + shell.top + shell.bottom,
                )
            }
            Node::Dot { side, .. }
            | Node::Logo { side, .. }
            | Node::Tick { side, .. }
            | Node::Badge { side, .. }
            | Node::Disc { side, .. }
            | Node::Star { side, .. } => (*side, *side),
            Node::Track { .. } => (0.0, 10.0),
            Node::Cover { wide, high, .. } => (*wide, *high),
            Node::Rule => (0.0, 1.0),
            Node::Grow => (0.0, 0.0),
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, given: f32) {
        let (own_wide, own_high) = self.measure();
        match self {
            Node::Row {
                gap,
                cross,
                wide,
                items,
            } => {
                let room = wide.unwrap_or(if self.stretches() { given } else { own_wide });
                let sizes: Vec<(f32, f32)> = items.iter().map(Node::measure).collect();
                let natural = sizes.iter().map(|size| size.0).sum::<f32>()
                    + gap * items.len().saturating_sub(1) as f32;
                let grows = items
                    .iter()
                    .filter(|item| matches!(item, Node::Grow))
                    .count();
                let spare = if grows > 0 {
                    (room - natural).max(0.0) / grows as f32
                } else {
                    0.0
                };
                let mut pen = x;
                for (item, size) in items.iter().zip(&sizes) {
                    let top = match cross {
                        Cross::Start => y,
                        Cross::Centre => y + (own_high - size.1) / 2.0,
                        Cross::End => y + own_high - size.1,
                    };
                    item.draw(canvas, pen, top, size.0);
                    pen += size.0
                        + gap
                        + if matches!(item, Node::Grow) {
                            spare
                        } else {
                            0.0
                        };
                }
            }
            Node::Col {
                gap,
                cross,
                wide,
                items,
            } => {
                let room = wide.unwrap_or(if self.stretches() {
                    given.max(own_wide)
                } else {
                    own_wide
                });
                let mut pen = y;
                for item in items {
                    let size = item.measure();
                    let (left, width) = if item.stretches() {
                        (x, room)
                    } else {
                        match cross {
                            Cross::Start => (x, size.0),
                            Cross::Centre => (x + (room - size.0) / 2.0, size.0),
                            Cross::End => (x + room - size.0, size.0),
                        }
                    };
                    item.draw(canvas, left, pen, width);
                    pen += size.1 + gap;
                }
            }
            Node::Text {
                text,
                face,
                size,
                colour,
            } => canvas.text(x, y, text, *face, *size, *colour),
            Node::Plate { shell, child, .. } => {
                canvas.plate(
                    x,
                    y,
                    own_wide,
                    own_high,
                    shell.radius,
                    shell.fill,
                    shell.edge,
                );
                child.draw(
                    canvas,
                    x + shell.side,
                    y + shell.top,
                    own_wide - shell.side * 2.0,
                );
            }
            Node::Morph {
                shell,
                from,
                to,
                shown,
            } => {
                let k = shown.clamp(0.0, 1.0);
                canvas.plate(
                    x,
                    y,
                    own_wide,
                    own_high,
                    shell.radius,
                    shell.fill,
                    shell.edge,
                );
                let (inner_wide, inner_high) = (
                    own_wide - shell.side * 2.0,
                    own_high - shell.top - shell.bottom,
                );
                for (side, alpha) in [(from, 1.0 - k), (to, k)] {
                    let room = side.measure().0.max(inner_wide);
                    canvas.layer(
                        x + shell.side,
                        y + shell.top,
                        inner_wide,
                        inner_high,
                        alpha,
                        |layer| side.draw(layer, 0.0, 0.0, room),
                    );
                }
            }
            Node::Dot { side, colour, glow } => {
                let (cx, cy) = (x + side / 2.0, y + side / 2.0);
                canvas.disc(
                    cx,
                    cy,
                    side / 2.0 + 4.0,
                    [colour[0], colour[1], colour[2], 0.2 * glow],
                );
                canvas.disc(
                    cx,
                    cy,
                    side / 2.0,
                    [colour[0], colour[1], colour[2], colour[3] * glow],
                );
            }
            Node::Logo { side, colour } => canvas.logo(x, y, *side, *colour),
            Node::Tick { side, colour } => {
                canvas.disc(
                    x + side / 2.0,
                    y + side / 2.0,
                    side / 2.0,
                    [colour[0], colour[1], colour[2], 0.2],
                );
                let unit = side / 16.0;
                canvas.line(
                    &[
                        (x + 4.6 * unit, y + 8.4 * unit),
                        (x + 7.1 * unit, y + 10.8 * unit),
                        (x + 11.4 * unit, y + 5.6 * unit),
                    ],
                    1.6 * unit,
                    *colour,
                );
            }
            Node::Badge {
                side,
                radius,
                colour,
                sign,
            } => {
                canvas.plate(
                    x,
                    y,
                    *side,
                    *side,
                    *radius,
                    [colour[0], colour[1], colour[2], 0.15],
                    None,
                );
                if sign == "✓" {
                    let unit = side / 16.0;
                    canvas.line(
                        &[
                            (x + 4.8 * unit, y + 8.4 * unit),
                            (x + 7.2 * unit, y + 10.8 * unit),
                            (x + 11.2 * unit, y + 5.8 * unit),
                        ],
                        1.3 * unit,
                        *colour,
                    );
                } else {
                    let size = side * 0.47;
                    let wide = text_width(sign, Face::MonoBold, size);
                    canvas.text(
                        x + (side - wide) / 2.0,
                        y + (side - line_height(size)) / 2.0,
                        sign,
                        Face::MonoBold,
                        size,
                        *colour,
                    );
                }
            }
            Node::Disc {
                side,
                colour,
                sign,
                ring,
            } => {
                if let Some(ring) = ring {
                    canvas.disc(x + side / 2.0, y + side / 2.0, side / 2.0 + 2.0, *ring);
                }
                canvas.disc(x + side / 2.0, y + side / 2.0, side / 2.0, *colour);
                let size = side * 0.5;
                let wide = text_width(sign, Face::Semi, size);
                canvas.text(
                    x + (side - wide) / 2.0,
                    y + (side - line_height(size)) / 2.0,
                    sign,
                    Face::Semi,
                    size,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
            Node::Star { side, colour } => {
                let (cx, cy, outer, inner) =
                    (x + side / 2.0, y + side / 2.0, side / 2.0, side / 4.6);
                let points: Vec<(f32, f32)> = (0..10)
                    .map(|at| {
                        let turn = std::f32::consts::PI * (at as f32 / 5.0 - 0.5);
                        let reach = if at % 2 == 0 { outer } else { inner };
                        (cx + reach * turn.cos(), cy + reach * turn.sin())
                    })
                    .collect();
                canvas.shape(&points, *colour);
            }
            Node::Track { share, mark, fill } => {
                canvas.plate(x, y + 3.0, given, 4.0, 2.0, [1.0, 1.0, 1.0, 0.14], None);
                canvas.plate(
                    x,
                    y + 3.0,
                    given * share.clamp(0.0, 1.0),
                    4.0,
                    2.0,
                    *fill,
                    None,
                );
                if let Some(mark) = mark {
                    canvas.plate(
                        x + (given * mark.clamp(0.0, 1.0) - 1.0).max(0.0),
                        y,
                        2.0,
                        10.0,
                        0.0,
                        [0.925, 0.906, 0.886, 1.0],
                        None,
                    );
                }
            }
            Node::Cover {
                wide,
                high,
                radius,
                from,
                to,
            } => {
                canvas.plate(x, y, *wide, *high, *radius, *to, None);
                canvas.plate(
                    x + wide * 0.08,
                    y + high * 0.1,
                    wide * 0.62,
                    high * 0.7,
                    high * 0.35,
                    [from[0], from[1], from[2], 0.75],
                    None,
                );
            }
            Node::Rule => canvas.plate(x, y, given, 1.0, 0.0, [1.0, 1.0, 1.0, 0.10], None),
            Node::Grow => {}
        }
    }
}
