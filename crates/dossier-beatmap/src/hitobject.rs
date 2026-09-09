pub const PLAYFIELD_WIDTH: f64 = 512.0;
pub const PLAYFIELD_HEIGHT: f64 = 384.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const CENTRE: Self = Self {
        x: PLAYFIELD_WIDTH / 2.0,
        y: PLAYFIELD_HEIGHT / 2.0,
    };

    pub fn distance_to(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    pub fn mirrored(self) -> Self {
        Self {
            x: self.x,
            y: PLAYFIELD_HEIGHT - self.y,
        }
    }

    pub fn flipped(self) -> Self {
        Self {
            x: PLAYFIELD_WIDTH - self.x,
            y: self.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    Bezier,

    Catmull,

    Linear,

    PerfectCircle,
}

impl CurveType {
    fn from_char(c: char) -> Option<Self> {
        Some(match c {
            'B' => Self::Bezier,
            'C' => Self::Catmull,
            'L' => Self::Linear,
            'P' => Self::PerfectCircle,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slider {
    pub curve_type: CurveType,

    pub points: Vec<Point>,

    pub slides: u32,

    pub length: f64,

    pub edge_sounds: Vec<u8>,

    pub edge_sets: Vec<(u8, u8)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    Circle,
    Slider(Slider),
    Spinner { end_time_ms: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HitObject {
    pub pos: Point,
    pub time_ms: f64,
    pub new_combo: bool,

    pub hit_sound: u8,

    pub hit_sample: HitSample,
    pub kind: ObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitSample {
    pub normal_set: u8,

    pub addition_set: u8,
    pub index: u32,

    pub volume: u8,
}

impl HitSample {
    pub(crate) fn parse(field: Option<&&str>) -> Self {
        let Some(text) = field else {
            return Self::default();
        };
        let mut parts = text.split(':');
        let mut next = |fallback: u32| -> u32 {
            parts
                .next()
                .and_then(|p| p.trim().parse().ok())
                .unwrap_or(fallback)
        };
        Self {
            normal_set: next(0) as u8,
            addition_set: next(0) as u8,
            index: next(0),
            volume: next(0).min(100) as u8,
        }
    }
}

impl HitObject {
    pub fn is_circle(&self) -> bool {
        matches!(self.kind, ObjectKind::Circle)
    }

    pub fn is_slider(&self) -> bool {
        matches!(self.kind, ObjectKind::Slider(_))
    }

    pub fn is_spinner(&self) -> bool {
        matches!(self.kind, ObjectKind::Spinner { .. })
    }

    pub fn end_time_ms(&self) -> f64 {
        match &self.kind {
            ObjectKind::Spinner { end_time_ms } => *end_time_ms,
            _ => self.time_ms,
        }
    }
}

pub mod sound_bits {
    pub const NORMAL: u8 = 1 << 0;
    pub const WHISTLE: u8 = 1 << 1;
    pub const FINISH: u8 = 1 << 2;
    pub const CLAP: u8 = 1 << 3;
}

pub(crate) mod type_bits {
    pub const CIRCLE: u32 = 1 << 0;
    pub const SLIDER: u32 = 1 << 1;
    pub const NEW_COMBO: u32 = 1 << 2;
    pub const SPINNER: u32 = 1 << 3;
    pub const MANIA_HOLD: u32 = 1 << 7;
}

pub(crate) fn parse_curve(spec: &str) -> Option<(CurveType, Vec<Point>)> {
    let mut parts = spec.split('|');
    let head = parts.next()?;
    let curve_type = CurveType::from_char(head.chars().next()?)?;

    let mut points = Vec::new();
    for token in parts {
        let (x, y) = token.split_once(':')?;
        points.push(Point {
            x: x.trim().parse().ok()?,
            y: y.trim().parse().ok()?,
        });
    }
    Some((curve_type, points))
}
