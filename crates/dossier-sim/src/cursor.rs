use dossier_beatmap::Point;
use dossier_replay::{Keys, ReplayFrame};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    pub pos: Point,
    pub keys: Keys,
}

#[derive(Debug)]
pub struct CursorTrack {
    frames: Vec<ReplayFrame>,
    buttons: Vec<Buttons>,
    hint: std::sync::atomic::AtomicUsize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons {
    pub down: Side,

    pub last: Side,
    pub last2: Side,

    pub left_edge: bool,
    pub right_edge: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Side(pub u8);

impl Side {
    pub const NONE: Self = Self(0);
    pub const LEFT: Self = Self(1);
    pub const RIGHT: Self = Self(2);
    pub const BOTH: Self = Self(3);

    pub fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub fn any(self) -> bool {
        self.0 != 0
    }
}

impl Clone for CursorTrack {
    fn clone(&self) -> Self {
        Self::new(self.frames.clone())
    }
}

impl CursorTrack {
    pub fn new(frames: Vec<ReplayFrame>) -> Self {
        let buttons = Self::button_states(&frames);
        Self {
            frames,
            buttons,
            hint: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn button_states(frames: &[ReplayFrame]) -> Vec<Buttons> {
        let mut out = Vec::with_capacity(frames.len());
        let mut state = Buttons::default();
        let mut previous = Side::NONE;
        for frame in frames {
            let mut now = Side::NONE;
            if frame.keys.0 & (Keys::M1 | Keys::K1) != 0 {
                now.0 |= Side::LEFT.0;
            }
            if frame.keys.0 & (Keys::M2 | Keys::K2) != 0 {
                now.0 |= Side::RIGHT.0;
            }
            state.left_edge = now.overlaps(Side::LEFT) && !previous.overlaps(Side::LEFT);
            state.right_edge = now.overlaps(Side::RIGHT) && !previous.overlaps(Side::RIGHT);
            if now != previous {
                state.last2 = state.last;
                state.last = state.down;
                state.down = now;
            }
            out.push(state);
            previous = now;
        }
        out
    }

    pub fn buttons_at(&self, time_ms: f64) -> Option<Buttons> {
        if self.frames.is_empty() {
            return None;
        }
        Some(self.buttons[self.index_at(time_ms)])
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn holds(&self) -> [Vec<(f64, f64)>; 2] {
        let [k1, k2] = self.spans([
            |keys: Keys| keys.contains(Keys::K1) || keys.contains(Keys::M1),
            |keys: Keys| keys.contains(Keys::K2) || keys.contains(Keys::M2),
        ]);
        [k1, k2]
    }

    pub fn holds_each(&self, lazer: bool) -> [Vec<(f64, f64)>; 4] {
        if lazer {
            return self.spans([
                |keys: Keys| keys.contains(Keys::M1),
                |keys: Keys| keys.contains(Keys::M2),
                |_: Keys| false,
                |_: Keys| false,
            ]);
        }
        self.spans([
            |keys: Keys| keys.contains(Keys::K1),
            |keys: Keys| keys.contains(Keys::K2),
            |keys: Keys| keys.contains(Keys::M1) && !keys.contains(Keys::K1),
            |keys: Keys| keys.contains(Keys::M2) && !keys.contains(Keys::K2),
        ])
    }

    fn spans<const N: usize>(&self, lanes: [fn(Keys) -> bool; N]) -> [Vec<(f64, f64)>; N] {
        let mut out: [Vec<(f64, f64)>; N] = std::array::from_fn(|_| Vec::new());
        let mut down = [None::<f64>; N];
        for frame in &self.frames {
            let at = frame.time_ms as f64;
            for (index, lane) in lanes.iter().enumerate() {
                match (down[index], lane(frame.keys)) {
                    (None, true) => down[index] = Some(at),
                    (Some(from), false) => {
                        out[index].push((from, at));
                        down[index] = None;
                    }
                    _ => {}
                }
            }
        }
        let last = self.frames.last().map_or(0.0, |f| f.time_ms as f64);
        for (index, from) in down.into_iter().enumerate() {
            if let Some(from) = from {
                out[index].push((from, last.max(from + 1.0)));
            }
        }
        out
    }

    pub fn frames(&self) -> &[ReplayFrame] {
        &self.frames
    }

    pub fn span_ms(&self) -> Option<(f64, f64)> {
        Some((
            self.frames.first()?.time_ms as f64,
            self.frames.last()?.time_ms as f64,
        ))
    }

    pub fn sample(&self, time_ms: f64) -> Option<Cursor> {
        if self.frames.is_empty() {
            return None;
        }
        let idx = self.index_at(time_ms);
        let current = &self.frames[idx];

        let Some(next) = self.frames.get(idx + 1) else {
            return Some(Cursor {
                pos: Point {
                    x: f64::from(current.x),
                    y: f64::from(current.y),
                },
                keys: current.keys,
            });
        };

        let span = (next.time_ms - current.time_ms) as f64;
        let t = if span > 0.0 {
            ((time_ms - current.time_ms as f64) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };

        Some(Cursor {
            pos: Point {
                x: lerp(f64::from(current.x), f64::from(next.x), t),
                y: lerp(f64::from(current.y), f64::from(next.y), t),
            },

            keys: current.keys,
        })
    }

    fn index_at(&self, time_ms: f64) -> usize {
        let last = self.frames.len() - 1;
        if time_ms <= self.frames[0].time_ms as f64 {
            self.hint.store(0, std::sync::atomic::Ordering::Relaxed);
            return 0;
        }
        if time_ms >= self.frames[last].time_ms as f64 {
            self.hint.store(last, std::sync::atomic::Ordering::Relaxed);
            return last;
        }

        let hint = self
            .hint
            .load(std::sync::atomic::Ordering::Relaxed)
            .min(last);
        if self.frames[hint].time_ms as f64 <= time_ms {
            let mut i = hint;
            for _ in 0..8 {
                match self.frames.get(i + 1) {
                    Some(next) if (next.time_ms as f64) <= time_ms => i += 1,
                    _ => break,
                }
            }
            if self
                .frames
                .get(i + 1)
                .is_some_and(|n| (n.time_ms as f64) > time_ms)
            {
                self.hint.store(i, std::sync::atomic::Ordering::Relaxed);
                return i;
            }
        }

        let idx = self
            .frames
            .partition_point(|f| (f.time_ms as f64) <= time_ms)
            .saturating_sub(1);
        self.hint.store(idx, std::sync::atomic::Ordering::Relaxed);
        idx
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
