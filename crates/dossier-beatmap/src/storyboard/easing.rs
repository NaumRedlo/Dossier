const HALF_PI: f64 = 1.570_796_370_506_286_6;
const PI: f64 = 3.141_592_741_012_573_2;
const TAU: f64 = 6.283_185_482_025_146_5;

#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn ease(kind: u8, time: f64, begin: f64, change: f64, duration: f64) -> f64 {
    let (mut t, b, c, d) = (time, begin, change, duration);
    if c == 0.0 || t == 0.0 || d == 0.0 {
        return b;
    }
    if t == d {
        return b + c;
    }
    match kind {
        2 | 3 => {
            t /= d;
            c * t * t + b
        }
        1 | 4 => {
            t /= d;
            -c * t * (t - 2.0) + b
        }
        5 => {
            t /= d / 2.0;
            if t < 1.0 {
                c / 2.0 * t * t + b
            } else {
                t -= 1.0;
                -c / 2.0 * (t * (t - 2.0) - 1.0) + b
            }
        }

        6 => {
            t /= d;
            c * t * t * t + b
        }
        7 => {
            t = t / d - 1.0;
            c * (t * t * t + 1.0) + b
        }
        8 => {
            t /= d / 2.0;
            if t < 1.0 {
                c / 2.0 * t * t * t + b
            } else {
                t -= 2.0;
                c / 2.0 * (t * t * t + 2.0) + b
            }
        }

        9 => {
            t /= d;
            c * t * t * t * t + b
        }
        10 => {
            t = t / d - 1.0;
            -c * (t * t * t * t - 1.0) + b
        }
        11 => {
            t /= d / 2.0;
            if t < 1.0 {
                c / 2.0 * t * t * t * t + b
            } else {
                t -= 2.0;
                -c / 2.0 * (t * t * t * t - 2.0) + b
            }
        }

        12 => {
            t /= d;
            c * t * t * t * t * t + b
        }
        13 => {
            t = t / d - 1.0;
            c * (t * t * t * t * t + 1.0) + b
        }
        14 => {
            t /= d / 2.0;
            if t < 1.0 {
                c / 2.0 * t * t * t * t * t + b
            } else {
                t -= 2.0;
                c / 2.0 * (t * t * t * t * t + 2.0) + b
            }
        }

        15 => -c * (t / d * HALF_PI).cos() + c + b,
        16 => c * (t / d * HALF_PI).sin() + b,
        17 => -c / 2.0 * ((PI * t / d).cos() - 1.0) + b,

        18 => c * 2f64.powf(10.0 * (t / d - 1.0)) + b,
        19 => c * (-(2f64.powf(-10.0 * t / d)) + 1.0) + b,
        20 => {
            t /= d / 2.0;
            if t < 1.0 {
                c / 2.0 * 2f64.powf(10.0 * (t - 1.0)) + b
            } else {
                t -= 1.0;
                c / 2.0 * (-(2f64.powf(-10.0 * t)) + 2.0) + b
            }
        }

        21 => {
            t /= d;
            -c * ((1.0 - t * t).sqrt() - 1.0) + b
        }
        22 => {
            t = t / d - 1.0;
            c * (1.0 - t * t).sqrt() + b
        }
        23 => {
            t /= d / 2.0;
            if t < 1.0 {
                -c / 2.0 * ((1.0 - t * t).sqrt() - 1.0) + b
            } else {
                t -= 2.0;
                c / 2.0 * ((1.0 - t * t).sqrt() + 1.0) + b
            }
        }

        24 => {
            t /= d;
            if t == 1.0 {
                return b + c;
            }
            let period = d * 0.3;
            let (amplitude, shift) = elastic(c, period);
            t -= 1.0;
            -(amplitude * 2f64.powf(10.0 * t) * ((t * d - shift) * TAU / period).sin()) + b
        }
        25 => {
            t /= d;
            if t == 1.0 {
                return b + c;
            }
            let period = d * 0.3;
            let (amplitude, shift) = elastic(c, period);
            amplitude * 2f64.powf(-10.0 * t) * ((t * d - shift) * TAU / period).sin() + c + b
        }

        26 => {
            t /= d;
            if t == 1.0 {
                return b + c;
            }
            let period = d * 0.3;
            let (amplitude, shift) = elastic(c, period);
            amplitude * 2f64.powf(-10.0 * t) * ((0.5 * t * d - shift) * TAU / period).sin() + c + b
        }
        27 => {
            t /= d;
            if t == 1.0 {
                return b + c;
            }
            let period = d * 0.3;
            let (amplitude, shift) = elastic(c, period);
            amplitude * 2f64.powf(-10.0 * t) * ((0.25 * t * d - shift) * TAU / period).sin() + c + b
        }
        28 => {
            t /= d / 2.0;
            if t == 2.0 {
                return b + c;
            }
            let period = d * 0.449_999_999_999_999_96;
            let (amplitude, shift) = elastic(c, period);
            t -= 1.0;
            if t < 0.0 {
                -0.5 * (amplitude * 2f64.powf(10.0 * t) * ((t * d - shift) * TAU / period).sin())
                    + b
            } else {
                amplitude * 2f64.powf(-10.0 * t) * ((t * d - shift) * TAU / period).sin() * 0.5
                    + c
                    + b
            }
        }

        29 => {
            t /= d;
            c * t * t * ((BACK + 1.0) * t - BACK) + b
        }
        30 => {
            t = t / d - 1.0;
            c * (t * t * ((BACK + 1.0) * t + BACK) + 1.0) + b
        }
        31 => {
            let mut back = BACK;
            t /= d / 2.0;
            if t < 1.0 {
                back *= 1.525;
                c / 2.0 * (t * t * ((back + 1.0) * t - back)) + b
            } else {
                back *= 1.525;
                t -= 2.0;
                c / 2.0 * (t * t * ((back + 1.0) * t + back) + 2.0) + b
            }
        }

        32 => c - ease(33, d - t, 0.0, c, d) + b,
        33 => {
            t /= d;
            if t < 1.0 / 2.75 {
                c * (7.5625 * t * t) + b
            } else if t < 2.0 / 2.75 {
                t -= 1.5 / 2.75;
                c * (7.5625 * t * t + 0.75) + b
            } else if t < 2.5 / 2.75 {
                t -= 2.25 / 2.75;
                c * (7.5625 * t * t + 0.9375) + b
            } else {
                t -= 2.625 / 2.75;
                c * (7.5625 * t * t + 63.0 / 64.0) + b
            }
        }
        34 => {
            if t < d / 2.0 {
                ease(32, t * 2.0, 0.0, c, d) * 0.5 + b
            } else {
                ease(33, t * 2.0 - d, 0.0, c, d) * 0.5 + c * 0.5 + b
            }
        }

        _ => c * (t / d) + b,
    }
}

const BACK: f64 = 1.701_58;

fn elastic(change: f64, period: f64) -> (f64, f64) {
    let amplitude = change;
    if amplitude < change.abs() {
        (change, period / 4.0)
    } else {
        (amplitude, period / TAU * (change / amplitude).asin())
    }
}
