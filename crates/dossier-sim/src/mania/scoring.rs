use super::judge::{Combo, Event, Part, ResultKind as R};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LazerScore {
    pub accuracy: f64,
    pub total_score_without_mods: u64,
}
impl Default for LazerScore {
    fn default() -> Self {
        Self {
            accuracy: 1.0,
            total_score_without_mods: 0,
        }
    }
}

pub(super) struct Accumulator {
    total: usize,
    judged: usize,
    base_score: u64,
    combo_portion: f64,
    maximum_combo_portion: f64,
    snapshot: LazerScore,
}
impl Accumulator {
    pub(super) fn new(total: usize) -> Self {
        let maximum_combo_portion = (1..=total).map(|n| 300.0 * combo_factor(n as u32)).sum();
        Self {
            total,
            judged: 0,
            base_score: 0,
            combo_portion: 0.0,
            maximum_combo_portion,
            snapshot: LazerScore::default(),
        }
    }
    pub(super) fn apply(&mut self, event: &Event, combo: Combo) -> LazerScore {
        if event.part == Part::HoldBreak {
            return self.snapshot;
        }
        let weight = match event.result {
            R::Perfect => 305,
            R::Great => 300,
            R::Good => 200,
            R::Ok => 100,
            R::Meh => 50,
            R::Miss => 0,
        };
        self.judged += 1;
        self.base_score += weight;
        let combo_weight = if event.result == R::Perfect {
            300
        } else {
            weight
        };
        self.combo_portion += combo_weight as f64 * combo_factor(combo.current);
        let accuracy = self.base_score as f64 / (305.0 * self.judged as f64);
        let combo_progress = self.combo_portion / self.maximum_combo_portion;
        let accuracy_progress = self.judged as f64 / self.total as f64;
        let score = 150_000.0 * combo_progress
            + 850_000.0 * accuracy.powf(2.0 + 2.0 * accuracy) * accuracy_progress;
        self.snapshot = LazerScore {
            accuracy,
            total_score_without_mods: score.round_ties_even() as u64,
        };
        self.snapshot
    }
}

fn combo_factor(combo: u32) -> f64 {
    (f64::from(combo).ln() / 4.0f64.ln()).clamp(0.5, 400.0f64.ln() / 4.0f64.ln())
}
