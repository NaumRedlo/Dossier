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
    travelled: Vec<f64>,
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
        let mut travelled = Vec::with_capacity(frames.len());
        let mut walked = 0.0f64;
        for (at, frame) in frames.iter().enumerate() {
            if at > 0 {
                let before = &frames[at - 1];
                walked += f64::from(frame.x - before.x).hypot(f64::from(frame.y - before.y));
            }
            travelled.push(walked);
        }
        Self {
            frames,
            buttons,
            travelled,
            hint: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn travelled_at(&self, time_ms: f64) -> Option<f64> {
        if self.frames.is_empty() {
            return None;
        }
        let at = self.index_at(time_ms);
        let base = self.travelled[at];
        let Some(next) = self.frames.get(at + 1) else {
            return Some(base);
        };
        let current = &self.frames[at];
        let span = (next.time_ms - current.time_ms) as f64;
        let t = if span > 0.0 { ((time_ms - current.time_ms as f64) / span).clamp(0.0, 1.0) } else { 0.0 };
        Some(base + (self.travelled[at + 1] - base) * t)
    }

    pub fn when_travelled(&self, distance: f64) -> Option<(f64, Point)> {
        if self.frames.is_empty() || distance < 0.0 {
            return None;
        }
        let at = self.travelled.partition_point(|walked| *walked < distance);
        let later = self.frames.get(at)?;
        let point = |frame: &ReplayFrame| Point { x: f64::from(frame.x), y: f64::from(frame.y) };
        if at == 0 {
            return Some((later.time_ms as f64, point(later)));
        }
        let earlier = &self.frames[at - 1];
        let (from, to) = (self.travelled[at - 1], self.travelled[at]);
        let t = if to > from { ((distance - from) / (to - from)).clamp(0.0, 1.0) } else { 0.0 };
        Some((
            lerp(earlier.time_ms as f64, later.time_ms as f64, t),
            Point { x: lerp(f64::from(earlier.x), f64::from(later.x), t), y: lerp(f64::from(earlier.y), f64::from(later.y), t) },
        ))
    }

    pub fn pressed_since(&self, time_ms: f64) -> Option<(bool, f64)> {
        if self.frames.is_empty() {
            return None;
        }
        let at = self.index_at(time_ms);
        let down = self.frames[at].keys.is_pressed();
        let mut first = at;
        while first > 0 && self.frames[first - 1].keys.is_pressed() == down {
            first -= 1;
        }
        let since = if first == 0 { f64::INFINITY } else { (time_ms - self.frames[first].time_ms as f64).max(0.0) };
        Some((down, since))
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

#[cfg(test)]
mod travel {
    use super::*;

    fn at(time_ms: i32, x: f32, y: f32, keys: u8) -> ReplayFrame {
        ReplayFrame { time_ms: time_ms.into(), x, y, keys: Keys(keys) }
    }

    #[test]
    fn the_path_is_measured_once_and_read_back_both_ways() {
        let track = CursorTrack::new(vec![at(0, 0.0, 0.0, 0), at(10, 30.0, 40.0, 0), at(20, 30.0, 40.0, 0), at(30, 30.0, 100.0, 0)]);
        assert_eq!(track.travelled_at(5.0), Some(25.0));
        assert_eq!(track.travelled_at(30.0), Some(110.0));
        let (when, where_) = track.when_travelled(80.0).expect("a point on the path");
        assert!((when - 25.0).abs() < 1e-9);
        assert!((where_.y - 70.0).abs() < 1e-9);
        assert_eq!(track.when_travelled(500.0), None);
    }

    #[test]
    fn a_press_is_timed_from_its_first_frame() {
        let track = CursorTrack::new(vec![at(0, 0.0, 0.0, 0), at(10, 0.0, 0.0, Keys::K1), at(20, 0.0, 0.0, Keys::K1), at(40, 0.0, 0.0, 0)]);
        assert_eq!(track.pressed_since(25.0), Some((true, 15.0)));
        assert_eq!(track.pressed_since(45.0), Some((false, 5.0)));
        assert_eq!(track.pressed_since(5.0).map(|(down, since)| (down, since.is_infinite())), Some((false, true)));
    }
}
