use dossier_beatmap::{Point, SliderPath};

pub const TAIL_LENIENCY: f64 = -36.0;

const MAX_LENGTH: f64 = 100_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nested {
    Head,
    Tick,
    Repeat,
    Tail,
}

#[derive(Debug, Clone, Copy)]
pub struct NestedObject {
    pub kind: Nested,
    pub time_ms: f64,

    pub pos: Point,
}

pub fn nested_objects(
    path: &SliderPath,
    start_ms: f64,
    span_duration_ms: f64,
    spans: u32,
    tick_distance: f64,
    velocity: f64,
) -> Vec<NestedObject> {
    let mut out = Vec::new();
    let at = |progress: f64| {
        path.position_at(progress)
            .unwrap_or(Point { x: 0.0, y: 0.0 })
    };

    out.push(NestedObject {
        kind: Nested::Head,
        time_ms: start_ms,
        pos: at(0.0),
    });
    if span_duration_ms <= 0.0 || spans == 0 {
        return out;
    }

    let length = path.length().min(MAX_LENGTH);
    let tick_distance = tick_distance.clamp(0.0, length);

    let min_distance_from_end = velocity * 10.0;

    for span in 0..spans {
        let span_start = start_ms + f64::from(span) * span_duration_ms;
        let reversed = span % 2 == 1;

        if tick_distance > 0.0 {
            let mut ticks = Vec::new();
            let mut d = tick_distance;
            while d <= length {
                if d >= length - min_distance_from_end {
                    break;
                }
                let path_progress = d / length;
                let time_progress = if reversed {
                    1.0 - path_progress
                } else {
                    path_progress
                };
                ticks.push(NestedObject {
                    kind: Nested::Tick,
                    time_ms: span_start + time_progress * span_duration_ms,
                    pos: at(path_progress),
                });
                d += tick_distance;
                if ticks.len() >= 10_000 {
                    break;
                }
            }
            if reversed {
                ticks.reverse();
            }
            out.extend(ticks);
        }

        if span + 1 < spans {
            out.push(NestedObject {
                kind: Nested::Repeat,
                time_ms: span_start + span_duration_ms,
                pos: at(f64::from((span + 1) % 2)),
            });
        }
    }

    out.push(NestedObject {
        kind: Nested::Tail,
        time_ms: start_ms + f64::from(spans) * span_duration_ms,
        pos: at(f64::from(spans % 2)),
    });
    out
}

pub fn tick_distance(velocity: f64, beat_length_ms: f64, tick_rate: f64) -> f64 {
    if tick_rate <= 0.0 {
        return 0.0;
    }
    velocity * beat_length_ms / tick_rate
}
