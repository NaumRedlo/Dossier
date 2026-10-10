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
}
