use crate::hitobject::{CurveType, Point};

impl Point {
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    fn scale(self, k: f64) -> Self {
        Self {
            x: self.x * k,
            y: self.y * k,
        }
    }

    fn distance(self, other: Self) -> f64 {
        self.sub(other).length()
    }

    fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    fn lerp(self, other: Self, t: f64) -> Self {
        self.add(other.sub(self).scale(t))
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flattening {
    Stable { format_version: u32 },
    Lazer,
}

impl Flattening {
    pub const LATEST_STABLE: Self = Self::Stable { format_version: 14 };
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliderPath {
    points: Vec<Point>,

    cumulative: Vec<f64>,
    length: f64,
}

impl SliderPath {
    pub fn new(
        curve_type: CurveType,
        control_points: &[Point],
        expected_length: Option<f64>,
    ) -> Self {
        Self::flattened(curve_type, control_points, expected_length, Flattening::LATEST_STABLE)
    }

    pub fn flattened(
        curve_type: CurveType,
        control_points: &[Point],
        expected_length: Option<f64>,
        how: Flattening,
    ) -> Self {
        let control: Vec<Single> = control_points
            .iter()
            .filter(|p| p.is_finite())
            .map(|p| Single { x: p.x as f32, y: p.y as f32 })
            .collect();
        match how {
            Flattening::Stable { format_version } => {
                let mut points = client::stable(curve_type, &control, format_version);
                if let Some(target) = expected_length.filter(|target| target.is_finite() && *target > 0.0) {
                    client::cut_as_stable(&mut points, target);
                }
                Self::in_single_precision(points)
            }
            Flattening::Lazer => {
                let points = client::lazer(curve_type, &control);
                Self::from_polyline(points.into_iter().map(Single::wide).collect(), expected_length)
            }
        }
    }

    fn in_single_precision(points: Vec<Single>) -> Self {
        let mut cumulative = Vec::with_capacity(points.len());
        let mut total = 0.0f64;
        cumulative.push(0.0);
        for pair in points.windows(2) {
            total += f64::from(pair[0].distance(pair[1]));
            cumulative.push(total);
        }
        if points.is_empty() {
            cumulative.clear();
        }
        Self {
            points: points.into_iter().map(Single::wide).collect(),
            cumulative,
            length: total,
        }
    }

    pub fn piece_lengths(&self) -> Vec<f64> {
        self.cumulative.windows(2).map(|pair| f64::from((pair[1] - pair[0]) as f32)).collect()
    }

    pub fn duration_ms(&self, pixels_per_second: f64) -> f64 {
        if pixels_per_second <= 0.0 || !pixels_per_second.is_finite() {
            return 0.0;
        }
        self.cumulative
            .windows(2)
            .map(|pair| f64::from(1000.0f32 * (pair[1] - pair[0]) as f32) / pixels_per_second)
            .sum()
    }

    fn from_polyline(mut points: Vec<Point>, expected_length: Option<f64>) -> Self {
        points.retain(|p| p.is_finite());
        if points.is_empty() {
            return Self {
                points: Vec::new(),
                cumulative: Vec::new(),
                length: 0.0,
            };
        }

        let mut cumulative = Vec::with_capacity(points.len());
        let mut total = 0.0;
        cumulative.push(0.0);
        for pair in points.windows(2) {
            total += pair[0].distance(pair[1]);
            cumulative.push(total);
        }

        let mut path = Self {
            points,
            cumulative,
            length: total,
        };
        if let Some(target) = expected_length {
            path.trim_to(target);
        }
        path
    }

    fn trim_to(&mut self, target: f64) {
        if !target.is_finite() || target <= 0.0 {
            self.points.truncate(1);
            self.cumulative.truncate(1);
            self.length = 0.0;
            return;
        }
        if target > self.length {
            self.stretch_to(target);
            return;
        }
        if target == self.length {
            return;
        }

        let idx = self.cumulative.partition_point(|&d| d < target);
        let before = idx - 1;
        let span = self.cumulative[idx] - self.cumulative[before];
        let t = if span > 0.0 {
            (target - self.cumulative[before]) / span
        } else {
            0.0
        };
        let cut = self.points[before].lerp(self.points[idx], t);

        self.points.truncate(idx);
        self.cumulative.truncate(idx);
        self.points.push(cut);
        self.cumulative.push(target);
        self.length = target;
    }

    fn stretch_to(&mut self, target: f64) {
        let last = self.points.len() - 1;
        if last == 0 {
            return;
        }
        let from = self.points[last - 1];
        let step = self.points[last].sub(from);
        let step_length = step.length();
        if step_length <= 0.0 {
            return;
        }
        let reach = target - self.cumulative[last - 1];
        self.points[last] = from.add(step.scale(reach / step_length));
        self.cumulative[last] = target;
        self.length = target;
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        for point in &mut self.points {
            point.x += dx;
            point.y += dy;
        }
    }

    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn points(&self) -> &[Point] {
        &self.points
    }

    pub fn position_at(&self, progress: f64) -> Option<Point> {
        let first = *self.points.first()?;
        if self.length <= 0.0 {
            return Some(first);
        }
        let target = progress.clamp(0.0, 1.0) * self.length;

        let idx = self.cumulative.partition_point(|&d| d < target).max(1);
        let (before, after) = (idx - 1, idx.min(self.points.len() - 1));
        let span = self.cumulative[after] - self.cumulative[before];
        let t = if span > 0.0 {
            (target - self.cumulative[before]) / span
        } else {
            0.0
        };
        Some(self.points[before].lerp(self.points[after], t))
    }

    pub fn segment(&self, from: f64, to: f64) -> Option<(Point, &[Point], Point)> {
        let from = from.clamp(0.0, 1.0);
        let to = to.clamp(from, 1.0);
        if self.points.len() < 2 || to <= from {
            return None;
        }
        let (start, end) = (self.position_at(from)?, self.position_at(to)?);
        let (from_at, to_at) = (from * self.length, to * self.length);

        let first = self.cumulative.partition_point(|&d| d <= from_at);
        let last = self.cumulative.partition_point(|&d| d < to_at);
        let interior = if first < last {
            &self.points[first..last]
        } else {
            &[][..]
        };
        Some((start, interior, end))
    }

    pub fn position_on_whole_ms(&self, start_ms: f64, slide_ms: f64, slides: u32, time_ms: f64) -> Option<Point> {
        let first = *self.points.first()?;
        let pieces = self.points.len() - 1;
        if pieces == 0 || self.length <= 0.0 || slide_ms <= 0.0 {
            return Some(first);
        }
        let slides = slides.max(1) as usize;
        let piece = |at: usize| -> (f64, f64, Point, Point) {
            let (slide, local) = (at / pieces, at % pieces);
            let base = start_ms + slide as f64 * slide_ms;
            let reach = |index: usize| self.cumulative[index] / self.length * slide_ms;
            if slide % 2 == 0 {
                (base + reach(local), base + reach(local + 1), self.points[local], self.points[local + 1])
            } else {
                let (from, to) = (pieces - local, pieces - local - 1);
                (base + slide_ms - reach(from), base + slide_ms - reach(to), self.points[from], self.points[to])
            }
        };
        let total = pieces * slides;
        let (mut low, mut high) = (0usize, total - 1);
        while low < high {
            let middle = (low + high) / 2;
            if piece(middle).1 >= time_ms {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        let (from_ms, to_ms, from, to) = piece(low);
        let (from_ms, to_ms) = (from_ms.trunc(), to_ms.trunc());
        if to_ms == from_ms {
            return Some(to);
        }
        let done = 1.0 - (to_ms - time_ms) / (to_ms - from_ms);
        Some(from.lerp(to, done))
    }

    pub fn position_at_slide(&self, progress: f64, slides: u32) -> Option<Point> {
        let slides = slides.max(1) as f64;
        let p = progress.clamp(0.0, slides);
        let index = p.floor().min(slides - 1.0);
        let mut local = p - index;
        if (index as u64) % 2 == 1 {
            local = 1.0 - local;
        }
        self.position_at(local)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Single {
    x: f32,
    y: f32,
}

impl Single {
    fn wide(self) -> Point {
        Point { x: f64::from(self.x), y: f64::from(self.y) }
    }

    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }

    fn sub(self, other: Self) -> Self {
        Self { x: self.x - other.x, y: self.y - other.y }
    }

    fn scale(self, by: f32) -> Self {
        Self { x: self.x * by, y: self.y * by }
    }

    fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    fn distance(self, other: Self) -> f32 {
        other.sub(self).length()
    }

    fn lerp(self, other: Self, amount: f32) -> Self {
        Self { x: self.x + (other.x - self.x) * amount, y: self.y + (other.y - self.y) * amount }
    }
}

mod client {
    use super::{CurveType, Single};

    const CATMULL_STEPS: usize = 50;
    const OLD_BEZIER_STEPS: usize = 50;
    const BEZIER_FLAT: f32 = 0.25;
    const STABLE_ARC_STEP: f64 = 0.125;
    const STABLE_TURN: f64 = 6.283_185_482_025_146_5;
    const LAZER_ARC_TOLERANCE: f32 = 0.1;
    const CUT_SLACK: f64 = 0.0001;

    pub(super) fn stable(curve_type: CurveType, control: &[Single], format_version: u32) -> Vec<Single> {
        if control.len() < 2 {
            return control.to_vec();
        }
        match curve_type {
            CurveType::Catmull => catmull(control),
            CurveType::Bezier => stable_bezier(control, format_version),
            CurveType::PerfectCircle => {
                if control.len() > 3 {
                    return stable_bezier(control, format_version);
                }
                if let [a, b, c] = *control {
                    if !in_a_line(a, b, c) {
                        return stable_arc(a, b, c);
                    }
                }
                control.to_vec()
            }
            CurveType::Linear => control.to_vec(),
        }
    }

    pub(super) fn lazer(curve_type: CurveType, control: &[Single]) -> Vec<Single> {
        if control.len() < 2 {
            return control.to_vec();
        }
        match curve_type {
            CurveType::Catmull => catmull(control),
            CurveType::Bezier => bezier_runs(control, |at| at + 1 < control.len() && control[at] == control[at + 1]),
            CurveType::PerfectCircle => match *control {
                [a, b, c] => lazer_arc(a, b, c).unwrap_or_else(|| bezier(control)),
                _ => bezier_runs(control, |at| at + 1 < control.len() && control[at] == control[at + 1]),
            },
            CurveType::Linear => control.to_vec(),
        }
    }

    fn join(out: &mut Vec<Single>, piece: Vec<Single>) {
        let skip = usize::from(!out.is_empty());
        out.extend(piece.into_iter().skip(skip));
    }

    fn bezier_runs(control: &[Single], breaks_after: impl Fn(usize) -> bool) -> Vec<Single> {
        let mut out = Vec::new();
        let mut from = 0usize;
        let mut at = 0usize;
        while at < control.len() {
            let breaks = breaks_after(at);
            if breaks || at == control.len() - 1 {
                join(&mut out, bezier(&control[from..=at]));
                if breaks {
                    at += 1;
                }
                from = at;
            }
            at += 1;
        }
        out
    }

    fn stable_bezier(control: &[Single], format_version: u32) -> Vec<Single> {
        let count = control.len();
        if format_version > 6 {
            let breaks = |at: usize| at + 2 < count && control[at] == control[at + 1];
            let mut out = Vec::new();
            let mut from = 0usize;
            let mut at = 0usize;
            while at < count {
                let broke = breaks(at);
                if broke || at == count - 1 {
                    let run = &control[from..=at];
                    let piece = if format_version > 8 && run.len() == 2 {
                        run.to_vec()
                    } else if format_version > 8 && format_version < 10 {
                        old_bezier(run)
                    } else {
                        bezier(run)
                    };
                    join(&mut out, piece);
                    if broke {
                        at += 1;
                    }
                    from = at;
                }
                at += 1;
            }
            return out;
        }
        let mut out = Vec::new();
        let mut from = 0usize;
        for at in 0..count {
            if (at > 0 && control[at] == control[at - 1]) || at == count - 1 {
                join(&mut out, bezier(&control[from..=at]));
                from = at;
            }
        }
        out
    }

    fn is_flat(control: &[Single]) -> bool {
        (1..control.len().saturating_sub(1)).all(|at| {
            control[at - 1].sub(control[at].scale(2.0)).add(control[at + 1]).length_squared() <= BEZIER_FLAT
        })
    }

    fn halve(control: &[Single], left: &mut [Single], right: &mut [Single]) {
        let count = control.len();
        let mut middle = control.to_vec();
        for at in 0..count {
            left[at] = middle[0];
            right[count - at - 1] = middle[count - at - 1];
            for step in 0..count - at - 1 {
                middle[step] = middle[step].add(middle[step + 1]).scale(0.5);
            }
        }
    }

    fn bezier(control: &[Single]) -> Vec<Single> {
        let count = control.len();
        let mut out = Vec::new();
        if count == 0 {
            return out;
        }
        let origin = Single { x: 0.0, y: 0.0 };
        let mut waiting: Vec<Vec<Single>> = vec![control.to_vec()];
        let mut left = vec![origin; count * 2 - 1];
        while let Some(mut parent) = waiting.pop() {
            if is_flat(&parent) {
                let mut right = vec![origin; count];
                halve(&parent, &mut left[..count], &mut right);
                for at in 0..count - 1 {
                    left[count + at] = right[at + 1];
                }
                out.push(parent[0]);
                for at in 1..count - 1 {
                    let middle = 2 * at;
                    out.push(left[middle - 1].add(left[middle].scale(2.0)).add(left[middle + 1]).scale(0.25));
                }
                continue;
            }
            let mut right = vec![origin; count];
            halve(&parent, &mut left[..count], &mut right);
            parent.copy_from_slice(&left[..count]);
            waiting.push(right);
            waiting.push(parent);
        }
        out.push(control[count - 1]);
        out
    }

    fn old_bezier(control: &[Single]) -> Vec<Single> {
        let count = control.len();
        let steps = OLD_BEZIER_STEPS * count;
        (0..steps)
            .map(|step| {
                let mut work = control.to_vec();
                let amount = step as f32 / steps as f32;
                for level in 0..count {
                    for at in 0..count - level - 1 {
                        work[at] = work[at].lerp(work[at + 1], amount);
                    }
                }
                work[0]
            })
            .collect()
    }

    fn catmull(control: &[Single]) -> Vec<Single> {
        let mut out = Vec::new();
        for at in 0..control.len() - 1 {
            let before = if at >= 1 { control[at - 1] } else { control[at] };
            let from = control[at];
            let to = control[at + 1];
            let after = if at + 2 < control.len() { control[at + 2] } else { to.add(to.sub(from)) };
            let first = usize::from(!out.is_empty());
            for step in first..=CATMULL_STEPS {
                out.push(catmull_rom(before, from, to, after, step as f32 / CATMULL_STEPS as f32));
            }
        }
        out
    }

    fn catmull_rom(a: Single, b: Single, c: Single, d: Single, amount: f32) -> Single {
        let squared = amount * amount;
        let cubed = amount * squared;
        let axis = |a: f32, b: f32, c: f32, d: f32| {
            0.5 * (2.0 * b + (-a + c) * amount + (2.0 * a - 5.0 * b + 4.0 * c - d) * squared + (-a + 3.0 * b - 3.0 * c + d) * cubed)
        };
        Single { x: axis(a.x, b.x, c.x, d.x), y: axis(a.y, b.y, c.y, d.y) }
    }

    fn in_a_line(a: Single, b: Single, c: Single) -> bool {
        (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y) == 0.0
    }

    fn circle_through(a: Single, b: Single, c: Single) -> (Single, f32) {
        let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
        let (sa, sb, sc) = (a.length_squared(), b.length_squared(), c.length_squared());
        let centre = Single {
            x: (sa * (b.y - c.y) + sb * (c.y - a.y) + sc * (a.y - b.y)) / d,
            y: (sa * (c.x - b.x) + sb * (a.x - c.x) + sc * (b.x - a.x)) / d,
        };
        (centre, centre.distance(a))
    }

    fn stable_arc(a: Single, b: Single, c: Single) -> Vec<Single> {
        let (centre, radius) = circle_through(a, b, c);
        let angle_of = |p: Single| f64::from(p.y - centre.y).atan2(f64::from(p.x - centre.x));
        let from = angle_of(a);
        let mut through = angle_of(b);
        let mut to = angle_of(c);
        while through < from {
            through += STABLE_TURN;
        }
        while to < from {
            to += STABLE_TURN;
        }
        if through > to {
            to -= STABLE_TURN;
        }
        let long = ((to - from) * f64::from(radius)).abs();
        let steps = (long * STABLE_ARC_STEP) as i64;
        let mut out = vec![a];
        for step in 1..steps {
            let share = step as f64 / steps as f64;
            let angle = to * share + from * (1.0 - share);
            out.push(Single {
                x: (angle.cos() * f64::from(radius)) as f32 + centre.x,
                y: (angle.sin() * f64::from(radius)) as f32 + centre.y,
            });
        }
        out.push(c);
        out
    }

    fn lazer_arc(a: Single, b: Single, c: Single) -> Option<Vec<Single>> {
        let lean = (b.y - a.y) * (c.x - a.x) - (b.x - a.x) * (c.y - a.y);
        if lean.abs() <= 1e-3 {
            return None;
        }
        let (centre, radius) = circle_through(a, b, c);
        let (from_centre, to_centre) = (a.sub(centre), c.sub(centre));
        let start = f64::from(from_centre.y).atan2(f64::from(from_centre.x));
        let mut end = f64::from(to_centre.y).atan2(f64::from(to_centre.x));
        while end < start {
            end += std::f64::consts::TAU;
        }
        let mut direction = 1.0f64;
        let mut range = end - start;
        let across = c.sub(a);
        let turned = Single { x: across.y, y: -across.x };
        let towards = b.sub(a);
        if turned.x * towards.x + turned.y * towards.y < 0.0 {
            direction = -direction;
            range = std::f64::consts::TAU - range;
        }
        let points = if 2.0 * radius <= LAZER_ARC_TOLERANCE {
            2
        } else {
            let step = 2.0 * f64::from(1.0 - LAZER_ARC_TOLERANCE / radius).acos();
            ((range / step).ceil() as usize).clamp(2, 1000)
        };
        Some(
            (0..points)
                .map(|at| {
                    let angle = start + direction * (at as f64 / (points - 1) as f64) * range;
                    Single { x: angle.cos() as f32 * radius + centre.x, y: angle.sin() as f32 * radius + centre.y }
                })
                .collect(),
        )
    }

    pub(super) fn cut_as_stable(points: &mut Vec<Single>, target: f64) {
        let total: f64 = points.windows(2).map(|pair| f64::from(pair[0].distance(pair[1]))).sum();
        if total <= 0.0 {
            return;
        }
        let mut over = total - target;
        while points.len() >= 2 {
            let (from, to) = (points[points.len() - 2], points[points.len() - 1]);
            let long = from.distance(to);
            if f64::from(long) > over + CUT_SLACK {
                if to != from {
                    let towards = to.sub(from);
                    let unit = towards.scale(1.0 / towards.length());
                    let last = points.len() - 1;
                    points[last] = from.add(unit.scale(long - over as f32));
                }
                break;
            }
            points.pop();
            over -= f64::from(long);
        }
    }
}

#[cfg(test)]
mod whole_ms {
    use super::*;

    fn straight(length: f64) -> SliderPath {
        SliderPath::new(CurveType::Linear, &[Point { x: 0.0, y: 0.0 }, Point { x: length, y: 0.0 }], Some(length))
    }

    #[test]
    fn the_ball_runs_ahead_when_the_slide_ends_between_two_milliseconds() {
        let path = straight(100.0);
        let plain = path.position_at_slide(50.0 / 100.5, 1).expect("on the path");
        let whole = path.position_on_whole_ms(1000.0, 100.5, 1, 1050.0).expect("on the path");
        assert!((whole.x - 50.0).abs() < 1e-9, "the slide is taken to end at 1100, so half of a hundred milliseconds is half of the path: {}", whole.x);
        assert!(whole.x > plain.x, "which is ahead of where the unrounded slide has it: {} against {}", whole.x, plain.x);
    }

    #[test]
    fn a_piece_that_begins_and_ends_in_one_millisecond_is_at_its_end() {
        let bent = SliderPath::new(
            CurveType::Linear,
            &[Point { x: 0.0, y: 0.0 }, Point { x: 0.4, y: 0.0 }, Point { x: 0.4, y: 0.4 }, Point { x: 100.0, y: 0.4 }],
            None,
        );
        let at = bent.position_on_whole_ms(1000.0, bent.length(), 1, 1000.0).expect("on the path");
        assert!((at.x - 0.4).abs() < 1e-6 && at.y == 0.0, "the first piece is over within the millisecond the slider starts in: {at:?}");
    }

    #[test]
    fn a_slide_back_runs_the_pieces_the_other_way() {
        let path = straight(100.0);
        let out = path.position_on_whole_ms(0.0, 100.0, 2, 25.0).expect("on the path");
        let back = path.position_on_whole_ms(0.0, 100.0, 2, 125.0).expect("on the path");
        assert!((out.x - 25.0).abs() < 1e-9 && (back.x - 75.0).abs() < 1e-9, "{} then {}", out.x, back.x);
        let turn = path.position_on_whole_ms(0.0, 100.0, 2, 100.0).expect("on the path");
        assert!((turn.x - 100.0).abs() < 1e-9, "the turn belongs to the slide that reaches it: {}", turn.x);
    }
}
