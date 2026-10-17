use dossier_beatmap::{Beatmap, ObjectKind};

const INDIVIDUAL_DECAY_BASE: f64 = 0.125;
const OVERALL_DECAY_BASE: f64 = 0.30;
const RELEASE_THRESHOLD: f64 = 30.0;
const SECTION_LENGTH: f64 = 400.0;
const DECAY_WEIGHT: f64 = 0.9;
const DIFFICULTY_MULTIPLIER: f64 = 0.018;

#[derive(Debug, Clone, Copy)]
struct Note {
    start: f64,
    end: f64,
    column: usize,
}

fn decay(value: f64, delta_ms: f64, base: f64) -> f64 {
    value * base.powf(delta_ms / 1000.0)
}

fn definitely_bigger(a: f64, b: f64) -> bool {
    a - b > 1.0
}

fn legacy_sort<T: Copy>(keys: &mut [T], compare: impl Fn(&T, &T) -> std::cmp::Ordering) {
    use std::cmp::Ordering::{Greater, Less};
    let less = |a: &T, b: &T| compare(a, b) == Less;
    let swap_if_greater = |keys: &mut [T], a: usize, b: usize| {
        if a != b && compare(&keys[a], &keys[b]) == Greater {
            keys.swap(a, b);
        }
    };
    fn down_heap<T: Copy>(keys: &mut [T], mut i: usize, n: usize, lo: usize, less: &impl Fn(&T, &T) -> bool) {
        let d = keys[lo + i - 1];
        while i <= n >> 1 {
            let mut child = 2 * i;
            if child < n && less(&keys[lo + child - 1], &keys[lo + child]) {
                child += 1;
            }
            if !less(&d, &keys[lo + child - 1]) {
                break;
            }
            keys[lo + i - 1] = keys[lo + child - 1];
            i = child;
        }
        keys[lo + i - 1] = d;
    }
    fn intro<T: Copy>(keys: &mut [T], lo: usize, mut hi: usize, mut depth: u32, less: &impl Fn(&T, &T) -> bool, swap_if_greater: &impl Fn(&mut [T], usize, usize)) {
        while hi > lo {
            let size = hi - lo + 1;
            if size <= 16 {
                match size {
                    2 => swap_if_greater(keys, lo, hi),
                    3 => {
                        swap_if_greater(keys, lo, hi - 1);
                        swap_if_greater(keys, lo, hi);
                        swap_if_greater(keys, hi - 1, hi);
                    }
                    _ => {
                        for i in lo..hi {
                            let t = keys[i + 1];
                            let mut j = i as isize;
                            while j >= lo as isize && less(&t, &keys[j as usize]) {
                                keys[j as usize + 1] = keys[j as usize];
                                j -= 1;
                            }
                            keys[(j + 1) as usize] = t;
                        }
                    }
                }
                return;
            }
            if depth == 0 {
                let n = hi - lo + 1;
                for i in (1..=n >> 1).rev() {
                    down_heap(keys, i, n, lo, less);
                }
                for i in (2..=n).rev() {
                    keys.swap(lo, lo + i - 1);
                    down_heap(keys, 1, i - 1, lo, less);
                }
                return;
            }
            depth -= 1;
            let middle = lo + ((hi - lo) >> 1);
            swap_if_greater(keys, lo, middle);
            swap_if_greater(keys, lo, hi);
            swap_if_greater(keys, middle, hi);
            let pivot = keys[middle];
            keys.swap(middle, hi - 1);
            let (mut left, mut right) = (lo, hi - 1);
            while left < right {
                left += 1;
                while less(&keys[left], &pivot) {
                    left += 1;
                }
                right -= 1;
                while less(&pivot, &keys[right]) {
                    right -= 1;
                }
                if left >= right {
                    break;
                }
                keys.swap(left, right);
            }
            keys.swap(left, hi - 1);
            intro(keys, left + 1, hi, depth, less, swap_if_greater);
            if left == 0 {
                return;
            }
            hi = left - 1;
        }
    }
    let length = keys.len();
    if length < 2 {
        return;
    }
    let mut depth = 0;
    let mut n = length;
    while n >= 1 {
        depth += 1;
        n /= 2;
    }
    intro(keys, 0, length - 1, 2 * depth, &less, &swap_if_greater);
}

pub fn stars(beatmap: &Beatmap, clock_rate: f64) -> f64 {
    if beatmap.mode != 3 || !(clock_rate.is_finite() && clock_rate > 0.0) {
        return 0.0;
    }
    let columns = beatmap.difficulty.circle_size;
    if !columns.is_finite() || !(1.0..=18.0).contains(&columns) {
        return 0.0;
    }
    let mut notes: Vec<Note> = beatmap
        .objects
        .iter()
        .map(|object| {
            let end = match object.kind {
                ObjectKind::Hold { end_time_ms } => end_time_ms,
                _ => object.time_ms,
            };
            let column = (object.pos.x * columns / 512.0).floor().clamp(0.0, columns - 1.0) as usize;
            Note { start: object.time_ms, end, column }
        })
        .collect();
    notes.sort_by(|a, b| a.start.total_cmp(&b.start));
    legacy_sort(&mut notes, |a, b| (a.start.round() as i64).cmp(&(b.start.round() as i64)));
    if notes.len() < 2 {
        return 0.0;
    }
    let lanes = columns as usize;
    let mut start_times = vec![0.0; lanes];
    let mut end_times = vec![0.0; lanes];
    let mut individual_strains = vec![0.0; lanes];
    let mut individual_strain = 0.0f64;
    let mut overall_strain = 1.0f64;
    let mut peaks: Vec<f64> = Vec::new();
    let mut section_end = 0.0;
    let mut section_peak = 0.0f64;
    for at in 1..notes.len() {
        let note = notes[at];
        let previous = notes[at - 1];
        let start = note.start / clock_rate;
        let end = note.end / clock_rate;
        let previous_start = previous.start / clock_rate;
        let delta = start - previous_start;
        if at == 1 {
            section_end = (start / SECTION_LENGTH).ceil() * SECTION_LENGTH;
        }
        while start > section_end {
            peaks.push(section_peak);
            let offset = section_end - previous_start;
            section_peak = decay(individual_strain, offset, INDIVIDUAL_DECAY_BASE) + decay(overall_strain, offset, OVERALL_DECAY_BASE);
            section_end += SECTION_LENGTH;
        }
        let mut overlapping = false;
        let mut closest_end = (end - start).abs();
        let mut hold_factor = 1.0;
        let mut hold_addition = 0.0;
        for lane in 0..lanes {
            let later = definitely_bigger(start, start_times[lane]);
            overlapping |= definitely_bigger(end_times[lane], start) && definitely_bigger(end, end_times[lane]) && later;
            if definitely_bigger(end_times[lane], end) && later {
                hold_factor = 1.25;
            }
            closest_end = closest_end.min((end - end_times[lane]).abs());
        }
        if overlapping {
            hold_addition = 1.0 / (1.0 + (0.27 * (RELEASE_THRESHOLD - closest_end)).exp());
        }
        let column = note.column;
        individual_strains[column] = decay(individual_strains[column], start - start_times[column], INDIVIDUAL_DECAY_BASE);
        individual_strains[column] += 2.0 * hold_factor;
        individual_strain = if delta <= 1.0 { individual_strain.max(individual_strains[column]) } else { individual_strains[column] };
        overall_strain = decay(overall_strain, delta, OVERALL_DECAY_BASE);
        overall_strain += (1.0 + hold_addition) * hold_factor;
        start_times[column] = start;
        end_times[column] = end;
        section_peak = section_peak.max(individual_strain + overall_strain);
    }
    peaks.push(section_peak);
    peaks.retain(|peak| *peak > 0.0);
    peaks.sort_by(|a, b| b.total_cmp(a));
    let mut difficulty = 0.0;
    let mut weight = 1.0;
    for peak in peaks {
        difficulty += peak * weight;
        weight *= DECAY_WEIGHT;
    }
    difficulty * DIFFICULTY_MULTIPLIER
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hits {
    pub perfect: u32,
    pub great: u32,
    pub good: u32,
    pub ok: u32,
    pub meh: u32,
    pub miss: u32,
}

impl Hits {
    pub fn total(self) -> u32 {
        self.perfect + self.great + self.good + self.ok + self.meh + self.miss
    }

    pub fn accuracy(self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        (f64::from(self.perfect) * 320.0 + f64::from(self.great) * 300.0 + f64::from(self.good) * 200.0 + f64::from(self.ok) * 100.0 + f64::from(self.meh) * 50.0)
            / (f64::from(total) * 320.0)
    }
}

pub fn performance(stars: f64, hits: Hits, no_fail: bool, easy: bool) -> f64 {
    let total = f64::from(hits.total());
    let mut multiplier = 1.0;
    if no_fail {
        multiplier *= 0.75;
    }
    if easy {
        multiplier *= 0.5;
    }
    let value = 8.0 * (stars - 0.15).max(0.05).powf(2.2) * (5.0 * hits.accuracy() - 4.0).max(0.0) * (1.0 + 0.1 * (total / 1500.0).min(1.0));
    let pp = value * multiplier;
    if pp.is_finite() { pp.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(objects: &str) -> Beatmap {
        Beatmap::parse(&format!("osu file format v14\n[General]\nMode:3\n[Difficulty]\nCircleSize:4\nOverallDifficulty:8\n[HitObjects]\n{objects}")).unwrap()
    }

    #[test]
    fn the_legacy_sort_orders_like_any_sort_by_key() {
        let mut keys: Vec<(i64, usize)> = (0..500).map(|at| ((at * 7919 % 97) as i64, at)).collect();
        legacy_sort(&mut keys, |a, b| a.0.cmp(&b.0));
        assert!(keys.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert_eq!(keys.len(), 500);
    }

    #[test]
    fn a_denser_chart_is_harder_and_a_faster_clock_raises_the_stars() {
        let sparse: String = (0..200).map(|at| format!("{},192,{},1,0,0:0:0:0:\n", 64 + 128 * (at % 4), 1000 + at * 250)).collect();
        let dense: String = (0..200).map(|at| format!("{},192,{},1,0,0:0:0:0:\n", 64 + 128 * (at % 4), 1000 + at * 90)).collect();
        let (easy, hard) = (stars(&map(&sparse), 1.0), stars(&map(&dense), 1.0));
        assert!(easy > 0.0 && hard > easy, "{easy} {hard}");
        assert!(stars(&map(&sparse), 1.5) > easy);
    }

    #[test]
    fn performance_follows_accuracy_and_vanishes_below_eighty_percent() {
        let perfect = Hits { perfect: 1000, ..Hits::default() };
        let worse = Hits { perfect: 800, great: 150, good: 50, ..Hits::default() };
        let bad = Hits { meh: 1000, ..Hits::default() };
        assert!(performance(5.0, perfect, false, false) > performance(5.0, worse, false, false));
        assert_eq!(performance(5.0, bad, false, false), 0.0);
        assert!((performance(5.0, perfect, true, false) / performance(5.0, perfect, false, false) - 0.75).abs() < 1e-9);
    }
}
