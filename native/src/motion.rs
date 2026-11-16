use std::time::Duration;

use iced::animation::Easing;

pub const QUICK: Duration = Duration::from_millis(120);
pub const STATE: Duration = Duration::from_millis(200);
pub const ENTER: Duration = Duration::from_millis(320);
pub const PAGE: Duration = Duration::from_millis(450);
pub const CURVE: Easing = Easing::Custom(curve);
pub const RESPONSE: f32 = 0.28;
pub const SLOW: f32 = 0.12;
pub const SETTLED: f32 = 0.004;

const FIRST: (f32, f32) = (0.22, 0.9);
const SECOND: (f32, f32) = (0.28, 1.0);

fn along(t: f32, first: f32, second: f32) -> f32 {
    let left = 1.0 - t;
    3.0 * left * left * t * first + 3.0 * left * t * t * second + t * t * t
}

fn slope(t: f32, first: f32, second: f32) -> f32 {
    let left = 1.0 - t;
    3.0 * left * left * first + 6.0 * left * t * (second - first) + 3.0 * t * t * (1.0 - second)
}

pub fn curve(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let mut t = x;
    for _ in 0..8 {
        let steep = slope(t, FIRST.0, SECOND.0);
        if steep.abs() < 1e-5 {
            break;
        }
        t = (t - (along(t, FIRST.0, SECOND.0) - x) / steep).clamp(0.0, 1.0);
    }
    along(t, FIRST.1, SECOND.1)
}

#[cfg(test)]
mod tests {
    use super::curve;

    #[test]
    fn the_curve_starts_at_nothing_ends_whole_and_never_turns_back() {
        assert!(curve(0.0).abs() < 1e-4);
        assert!((curve(1.0) - 1.0).abs() < 1e-4);
        let mut before = 0.0;
        for step in 1..=100 {
            let now = curve(step as f32 / 100.0);
            assert!(now >= before - 1e-4, "{now} after {before} at step {step}");
            before = now;
        }
    }

    #[test]
    fn the_curve_is_ahead_of_a_straight_line_and_eases_out() {
        assert!(curve(0.25) > 0.6, "{}", curve(0.25));
        assert!(curve(0.5) > 0.9, "{}", curve(0.5));
    }
}
