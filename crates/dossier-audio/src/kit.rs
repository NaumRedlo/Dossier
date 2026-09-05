#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timbre {
    Click,

    Soft,

    Drum,

    Glass,

    Wood,
}

impl Timbre {
    pub const ALL: [Self; 5] = [Self::Click, Self::Soft, Self::Drum, Self::Glass, Self::Wood];

    pub fn name(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Soft => "soft",
            Self::Drum => "drum",
            Self::Glass => "glass",
            Self::Wood => "wood",
        }
    }

    pub(crate) fn recipe(self) -> Recipe {
        match self {
            Self::Click => Recipe {
                centre: 1_100.0,
                resonance: 2.6,
                body: 0.0,
                droop: 0.0,
                length: 1.0,
                attack_ms: 0.5,
                partials: 1,
            },

            Self::Soft => Recipe {
                centre: 620.0,
                resonance: 1.5,
                body: 0.20,
                droop: 0.0,
                length: 1.6,
                attack_ms: 4.0,
                partials: 1,
            },

            Self::Drum => Recipe {
                centre: 900.0,
                resonance: 1.8,
                body: 0.75,
                droop: 0.55,
                length: 2.2,
                attack_ms: 0.4,
                partials: 1,
            },

            Self::Glass => Recipe {
                centre: 1_800.0,
                resonance: 4.0,
                body: 0.95,
                droop: 0.0,
                length: 2.8,
                attack_ms: 1.0,
                partials: 3,
            },

            Self::Wood => Recipe {
                centre: 2_300.0,
                resonance: 6.0,
                body: 0.45,
                droop: 0.40,
                length: 0.55,
                attack_ms: 0.3,
                partials: 1,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Recipe {
    pub centre: f32,

    pub resonance: f32,

    pub body: f32,

    pub droop: f32,

    pub length: f32,

    pub attack_ms: f32,

    pub partials: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kit {
    pub timbre: Timbre,

    pub pitch: f32,

    pub decay: f32,

    pub level: f32,
}

impl Kit {
    pub fn of(timbre: Timbre) -> Self {
        Self {
            timbre,
            pitch: 1.0,
            decay: 1.0,
            level: 2.0,
        }
    }

    pub fn plain() -> Self {
        Self::of(Timbre::Click)
    }

    pub fn by_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        Timbre::ALL
            .into_iter()
            .find(|t| t.name() == lower)
            .map(Self::of)
    }
}

impl Default for Kit {
    fn default() -> Self {
        Self::plain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Voice;

    #[test]
    fn every_pack_can_be_named_and_found_again() {
        for timbre in Timbre::ALL {
            let found = Kit::by_name(timbre.name()).expect("named packs resolve");
            assert_eq!(found.timbre, timbre);
        }

        assert!(Kit::by_name("1984").is_none());
        assert!(Kit::by_name("nonsense").is_none());
    }

    #[test]
    fn the_packs_actually_sound_different_from_each_other() {
        let fingerprints: Vec<_> = Timbre::ALL
            .into_iter()
            .map(|t| {
                let rendered = Voice::Normal.render(&Kit::of(t));
                (rendered.len() / 100, zero_crossings(&rendered) / 20)
            })
            .collect();

        for (i, a) in fingerprints.iter().enumerate() {
            for (j, b) in fingerprints.iter().enumerate().skip(i + 1) {
                assert_ne!(
                    a,
                    b,
                    "{} and {} are the same sound",
                    Timbre::ALL[i].name(),
                    Timbre::ALL[j].name()
                );
            }
        }
    }

    #[test]
    fn glass_is_the_one_without_noise_in_it() {
        let glass = Voice::Normal.render(&Kit::of(Timbre::Glass));
        let click = Voice::Normal.render(&Kit::of(Timbre::Click));
        assert!(
            regularity(&glass) < regularity(&click),
            "glass should be the steadier waveform"
        );
    }

    #[test]
    fn wood_is_the_shortest_pack_and_glass_the_longest() {
        let length = |t: Timbre| Voice::Normal.render(&Kit::of(t)).len();
        assert!(length(Timbre::Wood) < length(Timbre::Click));
        assert!(length(Timbre::Glass) > length(Timbre::Click));
    }

    #[test]
    fn a_shorter_decay_makes_shorter_sounds() {
        let long = Voice::Normal.render(&Kit::plain());
        let short = Voice::Normal.render(&Kit {
            decay: 0.5,
            ..Kit::plain()
        });
        assert!(short.len() < long.len());
    }

    #[test]
    fn pitch_moves_the_whole_pack_together() {
        for timbre in Timbre::ALL {
            let low = Kit {
                pitch: 0.5,
                ..Kit::of(timbre)
            };
            let high = Kit {
                pitch: 2.0,
                ..Kit::of(timbre)
            };
            for voice in [Voice::Normal, Voice::Whistle, Voice::Clap, Voice::Tick] {
                let a = zero_crossings(&voice.render(&low));
                let b = zero_crossings(&voice.render(&high));
                assert!(b > a, "{}/{voice:?} ignored the pitch", timbre.name());
            }
        }
    }

    fn zero_crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count()
    }

    fn regularity(samples: &[f32]) -> f64 {
        let mut gaps = Vec::new();
        let mut last = 0usize;
        for (i, w) in samples.windows(2).enumerate() {
            if (w[0] < 0.0) != (w[1] < 0.0) {
                gaps.push((i - last) as f64);
                last = i;
            }
        }
        if gaps.len() < 3 {
            return f64::MAX;
        }
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        (gaps.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / gaps.len() as f64).sqrt()
    }
}
