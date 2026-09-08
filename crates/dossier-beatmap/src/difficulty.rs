#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difficulty {
    pub hp_drain: f64,
    pub circle_size: f64,
    pub overall_difficulty: f64,
    pub approach_rate: f64,
    pub slider_multiplier: f64,
    pub slider_tick_rate: f64,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self {
            hp_drain: 5.0,
            circle_size: 5.0,
            overall_difficulty: 5.0,
            approach_rate: 5.0,
            slider_multiplier: 1.4,
            slider_tick_rate: 1.0,
        }
    }
}

const GAMEFIELD_ROUNDING_ALLOWANCE: f64 = 1.00041;

pub fn difficulty_range(value: f64, min: f64, mid: f64, max: f64) -> f64 {
    if value > 5.0 {
        mid + (max - mid) * (value - 5.0) / 5.0
    } else if value < 5.0 {
        mid - (mid - min) * (5.0 - value) / 5.0
    } else {
        mid
    }
}

impl Difficulty {
    pub fn preempt_ms(&self) -> f64 {
        difficulty_range(self.approach_rate, 1800.0, 1200.0, 450.0)
    }

    pub fn fade_in_ms(&self) -> f64 {
        self.preempt_ms() * 2.0 / 3.0
    }

    pub fn hit_window_300(&self) -> f64 {
        difficulty_range(self.overall_difficulty, 80.0, 50.0, 20.0).trunc()
    }

    pub fn hit_window_100(&self) -> f64 {
        difficulty_range(self.overall_difficulty, 140.0, 100.0, 60.0).trunc()
    }

    pub fn hit_window_50(&self) -> f64 {
        difficulty_range(self.overall_difficulty, 200.0, 150.0, 100.0).trunc()
    }

    pub fn circle_radius(&self) -> f64 {
        (54.4 - 4.48 * self.circle_size) * GAMEFIELD_ROUNDING_ALLOWANCE
    }

    pub fn spins_per_second(&self) -> f64 {
        difficulty_range(self.overall_difficulty, 90.0, 150.0, 225.0) / 60.0
    }

    pub fn top_spins_per_second(&self) -> f64 {
        difficulty_range(self.overall_difficulty, 250.0, 380.0, 430.0) / 60.0
    }

    pub fn hard_rock(&self) -> Self {
        Self {
            hp_drain: (self.hp_drain * 1.4).min(10.0),
            circle_size: (self.circle_size * 1.3).min(10.0),
            overall_difficulty: (self.overall_difficulty * 1.4).min(10.0),
            approach_rate: (self.approach_rate * 1.4).min(10.0),
            ..*self
        }
    }

    pub fn easy(&self) -> Self {
        Self {
            hp_drain: self.hp_drain * 0.5,
            circle_size: self.circle_size * 0.5,
            overall_difficulty: self.overall_difficulty * 0.5,
            approach_rate: self.approach_rate * 0.5,
            ..*self
        }
    }
}
