use dossier_replay::{bits, Replay, Setting};

use super::Error;

#[derive(Debug, Clone, Copy)]
pub(super) struct ReplayOptions {
    pub clock_rate: f64,
    pub accuracy_challenge: bool,
    pub hard_rock: bool,
    pub stable: bool,
}

impl ReplayOptions {
    pub fn from_replay(replay: &Replay) -> Result<Self, Error> {
        let mods = replay.lazer_mods();
        if replay.game_version < 30_000_000 {
            if replay.mods.0 & !(bits::DOUBLE_TIME | bits::HARD_ROCK) != 0 || !mods.is_empty() {
                return Err(Error::UnsupportedMods);
            }
            return Ok(Self {
                clock_rate: if replay.mods.0 & bits::DOUBLE_TIME != 0 {
                    1.5
                } else {
                    1.0
                },
                accuracy_challenge: false,
                hard_rock: replay.mods.0 & bits::HARD_ROCK != 0,
                stable: true,
            });
        }
        if replay.mods.0 & !bits::DOUBLE_TIME != 0 {
            return Err(Error::UnsupportedMods);
        }
        let legacy_dt = replay.mods.0 & bits::DOUBLE_TIME != 0;
        let mut options = Self {
            clock_rate: if legacy_dt { 1.5 } else { 1.0 },
            accuracy_challenge: false,
            hard_rock: false,
            stable: false,
        };
        let mut seen_dt = false;
        for modifier in mods {
            match modifier.acronym.as_str() {
                "DT" if !seen_dt => {
                    seen_dt = true;
                    options.clock_rate = 1.5;
                    for (name, value) in &modifier.settings {
                        match (name.as_str(), value) {
                            ("speed_change", Setting::Number(rate))
                                if rate.is_finite() && (1.01..=2.0).contains(rate) =>
                            {
                                options.clock_rate = *rate;
                            }
                            ("adjust_pitch", Setting::Bool(_)) => {}
                            _ => return Err(Error::UnsupportedMods),
                        }
                    }
                }
                "AC" if !options.accuracy_challenge => {
                    options.accuracy_challenge = true;
                }
                _ => return Err(Error::UnsupportedMods),
            }
        }
        if legacy_dt && !mods.is_empty() && !seen_dt {
            return Err(Error::UnsupportedMods);
        }
        Ok(options)
    }
}
