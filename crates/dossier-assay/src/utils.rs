pub fn logistic(x: f64, midpoint_offset: f64, multiplier: f64, max_value: f64) -> f64 {
    max_value / (1.0 + (multiplier * (midpoint_offset - x)).exp())
}

pub fn logistic_of(exponent: f64, max_value: f64) -> f64 {
    max_value / (1.0 + exponent.exp())
}

pub fn reverse_lerp(x: f64, start: f64, end: f64) -> f64 {
    ((x - start) / (end - start)).clamp(0.0, 1.0)
}

pub fn smoothstep_bell_curve(x: f64) -> f64 {
    let x = 0.5 - (x - 0.5).abs();
    let x = (x * 2.0).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub fn smoothstep(x: f64, start: f64, end: f64) -> f64 {
    let x = ((x - start) / (end - start)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub fn smootherstep(x: f64, start: f64, end: f64) -> f64 {
    let x = ((x - start) / (end - start)).clamp(0.0, 1.0);
    x * x * x * (x * (6.0 * x - 15.0) + 10.0)
}

pub fn norm(p: f64, values: &[f64]) -> f64 {
    values.iter().map(|x| x.powf(p)).sum::<f64>().powf(1.0 / p)
}

pub fn milliseconds_to_bpm_at(ms: f64, delimiter: f64) -> f64 {
    60_000.0 / (ms * delimiter)
}

pub fn milliseconds_to_bpm(ms: f64) -> f64 {
    milliseconds_to_bpm_at(ms, 4.0)
}

pub fn bpm_to_milliseconds(bpm: f64) -> f64 {
    60_000.0 / 4.0 / bpm
}

pub use std::f64::consts::SQRT_2 as SQRT2;

pub fn erf(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    if x.is_nan() {
        return f64::NAN;
    }
    if x.is_infinite() {
        return x.signum();
    }
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let tau = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    let value = 1.0 - tau * (-x * x).exp();
    if x >= 0.0 {
        value
    } else {
        -value
    }
}

pub fn erf_inv(x: f64) -> f64 {
    if x <= -1.0 {
        return f64::NEG_INFINITY;
    }
    if x >= 1.0 {
        return f64::INFINITY;
    }
    if x == 0.0 {
        return 0.0;
    }
    const A: f64 = 0.147;
    let sign = x.signum();
    let x = x.abs();

    let ln = (1.0 - x * x).ln();
    let t1 = 2.0 / (std::f64::consts::PI * A) + ln / 2.0;
    let t2 = ln / A;
    let base = (t1 * t1 - t2).sqrt() - t1;

    let correction = if x >= 0.85 {
        ((x - 0.85) / 0.293).powi(8)
    } else {
        0.0
    };
    sign * (base.sqrt() + correction)
}
