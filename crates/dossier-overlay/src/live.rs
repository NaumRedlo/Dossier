pub fn unstable_rate(errors: &[i32], clock_rate: f64) -> Option<f64> {
    if errors.len() < 2 || !clock_rate.is_finite() || clock_rate <= 0.0 {
        return None;
    }
    let count = errors.len() as f64;
    let mean = errors.iter().map(|error| f64::from(*error)).sum::<f64>() / count;
    let spread = errors
        .iter()
        .map(|error| (f64::from(*error) - mean).powi(2))
        .sum::<f64>()
        / count;
    Some(spread.sqrt() * 10.0 / clock_rate)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rests {
    pub first: f64,
    pub last: f64,
    pub breaks: Vec<(f64, f64)>,
}

impl Rests {
    pub fn parse(text: &str) -> Option<Rests> {
        let mut section = "";
        let mut found = Rests::default();
        let mut objects = 0usize;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                section = line;
                continue;
            }
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            match section {
                "[Events]" if matches!(fields.first(), Some(&"2") | Some(&"Break")) => {
                    let from = fields.get(1).and_then(|field| field.parse::<f64>().ok());
                    let to = fields.get(2).and_then(|field| field.parse::<f64>().ok());
                    if let (Some(from), Some(to)) = (from, to) {
                        if from.is_finite() && to.is_finite() && to > from {
                            found.breaks.push((from, to));
                        }
                    }
                }
                "[HitObjects]" => {
                    let Some(time) = fields.get(2).and_then(|field| field.parse::<f64>().ok())
                    else {
                        continue;
                    };
                    let spins = fields
                        .get(3)
                        .and_then(|field| field.parse::<u32>().ok())
                        .is_some_and(|kind| kind & 8 != 0);
                    let end = if spins {
                        fields
                            .get(5)
                            .and_then(|field| field.split(':').next())
                            .and_then(|field| field.parse::<f64>().ok())
                            .unwrap_or(time)
                    } else {
                        time
                    };
                    if objects == 0 {
                        found.first = time;
                    }
                    found.last = found.last.max(end.max(time));
                    objects += 1;
                }
                _ => {}
            }
        }
        (objects > 0).then_some(found)
    }

    pub fn resting(&self, time_ms: f64) -> bool {
        resting(&self.breaks, self.first, self.last, time_ms)
    }
}

pub fn resting(breaks: &[(f64, f64)], first_object: f64, last_object: f64, time_ms: f64) -> bool {
    time_ms < first_object
        || time_ms > last_object
        || breaks
            .iter()
            .any(|(from, to)| time_ms >= *from && time_ms <= *to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unstable_rate_is_ten_times_the_spread_of_hit_errors_at_the_played_speed() {
        assert_eq!(unstable_rate(&[], 1.0), None);
        assert_eq!(unstable_rate(&[4], 1.0), None, "one hit has no spread");
        assert_eq!(unstable_rate(&[5, 5, 5, 5], 1.0), Some(0.0));
        let rate = unstable_rate(&[-10, 10, -10, 10], 1.0).unwrap();
        assert!((rate - 100.0).abs() < 1e-9, "{rate}");
        let faster = unstable_rate(&[-10, 10, -10, 10], 1.5).unwrap();
        assert!((faster - 100.0 / 1.5).abs() < 1e-9, "{faster}");
        assert_eq!(unstable_rate(&[1, 2], 0.0), None);
    }

    #[test]
    fn a_map_rests_before_its_first_note_in_its_breaks_and_after_its_last_note() {
        let breaks = [(20_000.0, 28_000.0)];
        let rests = |time: f64| resting(&breaks, 1_500.0, 90_000.0, time);
        assert!(rests(-800.0) && rests(1_000.0), "the lead-in is a rest");
        assert!(!rests(1_500.0) && !rests(19_999.0));
        assert!(rests(20_000.0) && rests(24_000.0) && rests(28_000.0));
        assert!(!rests(28_001.0) && !rests(90_000.0));
        assert!(rests(90_001.0), "the outro is a rest");
    }

    #[test]
    fn the_rests_of_a_map_are_read_from_its_file() {
        let text = "osu file format v14\n\n[Events]\n//Break Periods\n2,20000,28000\nBreak,40000,41000\n2,5,1\n0,0,\"bg.jpg\",0,0\n\n[HitObjects]\n256,192,1500,1,0,0:0:0:0:\n100,100,30000,6,0,B|200:200,1,100\n256,192,88000,12,0,90000,0:0:0:0:\n";
        let rests = Rests::parse(text).unwrap();
        assert_eq!((rests.first, rests.last), (1_500.0, 90_000.0));
        assert_eq!(rests.breaks, [(20_000.0, 28_000.0), (40_000.0, 41_000.0)]);
        assert!(rests.resting(24_000.0) && !rests.resting(30_000.0) && rests.resting(90_500.0));
        assert_eq!(
            Rests::parse("[Events]\n2,1,2\n"),
            None,
            "a file with no notes says nothing"
        );
    }
}
