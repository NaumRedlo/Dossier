mod scorers;
mod select;

pub use scorers::{Facet, Scorer};

use dossier_sim::GameState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub from_ms: f64,
    pub to_ms: f64,
}

impl Span {
    pub fn new(from_ms: f64, to_ms: f64) -> Self {
        Self { from_ms, to_ms }
    }

    pub fn length_ms(&self) -> f64 {
        self.to_ms - self.from_ms
    }

    pub fn centre_ms(&self) -> f64 {
        (self.from_ms + self.to_ms) / 2.0
    }

    pub fn overlaps(&self, other: &Span) -> bool {
        self.from_ms < other.to_ms && other.from_ms < self.to_ms
    }

    fn shifted_to(&self, from_ms: f64) -> Self {
        Self::new(from_ms, from_ms + self.length_ms())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reason {
    Kiai {
        bpm: f64,
        length_ms: f64,
    },

    Peak {
        combo: u32,
    },

    Choke {
        combo: u32,

        through: f64,
    },

    Storm {
        objects: usize,

        of_densest: f64,
    },

    Precision {
        clicks: usize,
        mean_error_ms: f64,

        baseline_ms: f64,
    },

    Scramble {
        misses: usize,
        refused: usize,
    },

    Brink {
        low: f64,

        recovered_to: f64,
    },

    Opening {
        objects: usize,
    },

    Finale {
        failed: bool,
        accuracy: f64,
        combo: u32,

        full_combo: bool,
    },

    Tapping {
        per_second: f64,

        of_hardest: f64,

        taps: usize,
    },

    Travel {
        speed: f64,

        of_fastest: f64,
    },
}

impl Reason {
    pub fn scorer(&self) -> Scorer {
        match self {
            Self::Kiai { .. } => Scorer::Kiai,
            Self::Peak { .. } => Scorer::Peak,
            Self::Choke { .. } => Scorer::Choke,
            Self::Storm { .. } => Scorer::Storm,
            Self::Precision { .. } => Scorer::Precision,
            Self::Scramble { .. } => Scorer::Scramble,
            Self::Brink { .. } => Scorer::Brink,
            Self::Opening { .. } => Scorer::Opening,
            Self::Finale { .. } => Scorer::Finale,
            Self::Travel { .. } => Scorer::Travel,
            Self::Tapping { .. } => Scorer::Tapping,
        }
    }

    pub fn describe(&self) -> String {
        match *self {
            Self::Kiai { bpm, length_ms } => {
                format!("kiai — {:.0}s the mapper marked, at {bpm:.0} BPM", length_ms / 1000.0)
            }
            Self::Peak { combo } => format!("the play's longest run, {combo}x, ends here"),
            Self::Choke { combo, through } => format!(
                "a {combo}x run breaks {:.0}% of the way in",
                through * 100.0
            ),
            Self::Storm {
                objects,
                of_densest,
            } if of_densest >= 0.999 => {
                format!("the densest stretch of the map, {objects} objects")
            }
            Self::Storm {
                objects,
                of_densest,
            } => format!(
                "a dense stretch, {objects} objects — {:.0}% of the map's busiest",
                of_densest * 100.0
            ),
            Self::Precision {
                clicks,
                mean_error_ms,
                baseline_ms,
            } => format!(
                "{clicks} clicks at {mean_error_ms:.1}ms average error, against {baseline_ms:.1}ms for the play"
            ),
            Self::Scramble { misses, refused } => match (misses, refused) {
                (m, 0) => format!("{m} misses together"),
                (0, r) => format!("{r} clicks the game refused"),
                (m, r) => format!("{m} misses and {r} refused clicks together"),
            },
            Self::Brink { low, recovered_to } => format!(
                "the bar falls to {low:.0}% and climbs back to {recovered_to:.0}%"
            ),
            Self::Opening { objects } => {
                format!("how the play opens, {objects} objects in")
            }
            Self::Finale {
                failed: true,
                accuracy,
                combo,
                ..
            } => format!("the play ends here — the bar empties at {combo}x, {accuracy:.2}%"),
            Self::Finale {
                accuracy,
                combo,
                full_combo: true,
                ..
            } => format!("it lands — {combo}x all the way, {accuracy:.2}%"),
            Self::Finale {
                accuracy, combo, ..
            } => format!("how it finishes — {combo}x, {accuracy:.2}%"),
            Self::Tapping {
                per_second,
                of_hardest,
                taps,
            } if of_hardest >= 0.999 => format!(
                "the hardest tapping in the play, {taps} presses at {per_second:.1} a second"
            ),
            Self::Tapping {
                per_second, taps, ..
            } => format!("hard tapping, {taps} presses at {per_second:.1} a second"),
            Self::Travel {
                speed,
                of_fastest,
            } if of_fastest >= 0.999 => {
                format!("the hardest movement in the play, {speed:.0} osu!px a second")
            }
            Self::Travel { speed, .. } => {
                format!("hard movement, {speed:.0} osu!px a second")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub anchor_ms: f64,

    pub bias: f64,

    pub strength: f64,
    pub reason: Reason,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clip {
    pub span: Span,
    pub reason: Reason,

    pub with: Option<Reason>,

    pub rank: usize,

    pub score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub budget_ms: f64,

    pub worth: f64,

    pub clip_ms: f64,

    pub spread: f64,

    pub stretch: f64,
}

const NO_CEILING_MS: f64 = 120_000.0;

impl Default for Settings {
    fn default() -> Self {
        Self {
            budget_ms: NO_CEILING_MS,

            worth: 0.25,
            clip_ms: 6_000.0,
            spread: 3.0,
            stretch: 0.75,
        }
    }
}

impl Settings {
    fn in_map_time(&self, rate: f64) -> Self {
        let rate = if rate > 0.0 { rate } else { 1.0 };
        Self {
            budget_ms: self.budget_ms * rate,

            worth: self.worth,
            clip_ms: self.clip_ms * rate,
            spread: self.spread,
            stretch: self.stretch,
        }
    }

    fn length_for(&self, score: f64) -> f64 {
        self.clip_ms * (1.0 + self.stretch.max(0.0) * score.clamp(0.0, 1.0))
    }
}

pub fn choose(state: &GameState, settings: Settings) -> Vec<Clip> {
    let settings = settings.in_map_time(state.playback_rate());
    let candidates = scorers::all(state, settings);
    select::choose(candidates, state.span_ms(), state.timeline(), settings)
}

pub fn candidates(state: &GameState, settings: Settings) -> Vec<(Scorer, Candidate)> {
    scorers::all(state, settings.in_map_time(state.playback_rate()))
}
