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
        let map = dossier_beatmap::Beatmap::parse(text).ok()?;
        if map.mode != 0 || map.objects.is_empty() {
            return None;
        }
        let timeline = dossier_sim::Timeline::build(&map, dossier_replay::Mods::new(0));
        let first = timeline.objects.iter().map(|object| object.start_ms).reduce(f64::min)?;
        let last = timeline.objects.iter().map(|object| object.end_ms).reduce(f64::max)?;
        if !first.is_finite() || !last.is_finite() {
            return None;
        }
        Some(Self { first, last, breaks: map.breaks })
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

#[cfg(test)]
mod slider_tests {
    use super::*;

    #[test]
    fn a_final_repeated_slider_finishes_after_all_spans_with_inherited_velocity() {
        let map = "osu file format v14\n[General]\nMode:0\n[Difficulty]\nSliderMultiplier:1\nSliderTickRate:1\n[TimingPoints]\n0,500,4,2,0,100,1,0\n0,-50,4,2,0,100,0,0\n[HitObjects]\n100,100,1000,2,0,L|300:100,2,200\n";
        let rests = Rests::parse(map).unwrap();
        assert!((rests.last - 2000.0).abs() < 1.0, "{}", rests.last);
        assert!(!rests.resting(1500.0));
        assert!(!rests.resting(rests.last));
        assert!(rests.resting(rests.last + 1.0));
    }
}
