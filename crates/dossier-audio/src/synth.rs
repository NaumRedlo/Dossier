use crate::kit::{Kit, Recipe};
use crate::SAMPLE_RATE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voice {
    Normal,

    Whistle,

    Finish,

    Clap,

    Tick,

    Bonus,

    Slide,
    SlideWhistle,
    Spin,

    Miss,
}

impl Voice {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Normal => "hitnormal",
            Self::Whistle => "hitwhistle",
            Self::Finish => "hitfinish",
            Self::Clap => "hitclap",
            Self::Tick => "slidertick",
            Self::Slide => "sliderslide",
            Self::SlideWhistle => "sliderwhistle",

            Self::Bonus => "spinnerbonus",
            Self::Spin => "spinnerspin",
            Self::Miss => "combobreak",
        }
    }

    pub fn banked(self) -> bool {
        !matches!(self, Self::Bonus | Self::Spin | Self::Miss)
    }

    pub fn render(self, kit: &Kit) -> Vec<f32> {
        let recipe = kit.timbre.recipe();
        let seconds = |base: f32| base * recipe.length * kit.decay;
        let hz = |multiple: f32| recipe.centre * multiple * kit.pitch;

        match self {
            Self::Normal => strike(seconds(0.038), hz(1.0), &recipe, 0x51ed_2701),

            Self::Whistle => ring(seconds(0.075), hz(1.5), recipe.partials),
            Self::Finish => splash(seconds(0.30), hz(0.55), &recipe),
            Self::Clap => clap(seconds(0.055), hz(1.35), &recipe),

            Self::Tick => strike(seconds(0.018), hz(2.2), &recipe, 0x1234_5678),

            Self::Slide | Self::SlideWhistle | Self::Spin => Vec::new(),

            Self::Bonus => ring(seconds(0.10), hz(2.6), recipe.partials),

            Self::Miss => {
                let body = strike(
                    seconds(0.34),
                    hz(0.30),
                    &Recipe {
                        body: (recipe.body + 0.55).min(1.0),
                        droop: 0.62,
                        resonance: recipe.resonance * 0.6,
                        ..recipe
                    },
                    0x00fa_11ed,
                );
                echo(body, seconds(0.115), 0.42, 3)
            }
        }
    }

    pub fn gain(self, kit: &Kit) -> f32 {
        let base = match self {
            Self::Normal => 0.62,
            Self::Whistle => 0.48,
            Self::Finish => 0.70,
            Self::Clap => 0.60,
            Self::Tick => 0.24,
            Self::Bonus => 0.34,

            Self::Slide => 0.30,
            Self::SlideWhistle => 0.24,
            Self::Spin => 0.34,

            Self::Miss => 0.40,
        };
        base * kit.level
    }
}

fn strike(seconds: f32, hz: f32, recipe: &Recipe, seed: u32) -> Vec<f32> {
    let mut rng = Noise::new(seed);
    let mut filter = Svf::new(hz, recipe.resonance);
    let noise_level = (1.0 - recipe.body) * 1.6;

    let body_hz = hz * 0.5;
    let mut phase = 0.0f32;

    envelope(seconds, recipe.attack_ms, |t, envelope| {
        let noisy = filter.band(rng.next()) * noise_level;

        let progress = (t / seconds).min(1.0);
        let current = body_hz * (1.0 - recipe.droop * progress);
        phase += current / SAMPLE_RATE as f32 * std::f32::consts::TAU;
        let body = phase.sin() * recipe.body;

        (noisy + body) * envelope
    })
}

fn echo(source: Vec<f32>, delay_seconds: f32, feedback: f32, taps: usize) -> Vec<f32> {
    let delay = (delay_seconds * SAMPLE_RATE as f32).max(1.0) as usize;
    let mut out = vec![0.0; source.len() + delay * taps];
    for (i, sample) in source.iter().enumerate() {
        out[i] += sample;
    }
    let mut gain = feedback;
    for tap in 1..=taps {
        let offset = delay * tap;
        for (i, sample) in source.iter().enumerate() {
            out[i + offset] += sample * gain;
        }
        gain *= feedback;
    }
    out
}

fn ring(seconds: f32, hz: f32, partials: usize) -> Vec<f32> {
    const RATIOS: [f32; 3] = [1.0, 2.76, 5.40];
    let count = partials.clamp(1, RATIOS.len());

    envelope(seconds, 1.0, |t, envelope| {
        let sum: f32 = RATIOS[..count]
            .iter()
            .enumerate()
            .map(|(i, ratio)| {
                let decay = (-t / seconds.max(0.005) * 2.0 * i as f32).exp();
                (t * hz * ratio * std::f32::consts::TAU).sin() * decay
            })
            .sum();
        sum / count as f32 * envelope
    })
}

fn splash(seconds: f32, hz: f32, recipe: &Recipe) -> Vec<f32> {
    let mut rng = Noise::new(0x9e37_79b9);
    let mut low = OnePole::new(0.72);
    let mut phase = 0.0f32;

    envelope(seconds, recipe.attack_ms, |_, envelope| {
        let raw = rng.next();

        let shimmer = (raw - low.step(raw)) * 2.4 * (1.0 - recipe.body * 0.5);
        phase += hz / SAMPLE_RATE as f32 * std::f32::consts::TAU;
        (shimmer + phase.sin() * recipe.body * 0.6) * envelope
    })
}

fn clap(seconds: f32, hz: f32, recipe: &Recipe) -> Vec<f32> {
    let gap = (0.007 * SAMPLE_RATE as f32) as usize;
    let mut out = strike(seconds, hz, recipe, 0xa5a5_1234);

    let first = out.clone();
    for (i, value) in first.iter().enumerate() {
        if let Some(slot) = out.get_mut(i + gap) {
            *slot += value * 0.8;
        }
    }
    out
}

fn envelope(seconds: f32, attack_ms: f32, mut voice: impl FnMut(f32, f32) -> f32) -> Vec<f32> {
    let samples = (seconds * SAMPLE_RATE as f32) as usize;
    let attack_rate = 1_000.0 / attack_ms.max(0.05);
    (0..samples)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;

            let attack = (t * attack_rate).min(1.0);
            let decay = (-t / (seconds * 0.30)).exp();
            voice(t, attack * decay)
        })
        .collect()
}

struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

struct Svf {
    f: f32,
    damping: f32,
    low: f32,
    band: f32,
}

impl Svf {
    fn new(hz: f32, resonance: f32) -> Self {
        let hz = hz.clamp(20.0, SAMPLE_RATE as f32 * 0.45);
        Self {
            f: 2.0 * (std::f32::consts::PI * hz / SAMPLE_RATE as f32).sin(),
            damping: 1.0 / resonance.max(0.5),
            low: 0.0,
            band: 0.0,
        }
    }

    fn band(&mut self, input: f32) -> f32 {
        let high = input - self.low - self.damping * self.band;
        self.band += self.f * high;
        self.low += self.f * self.band;
        self.band
    }
}

struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(coefficient: f32) -> Self {
        Self {
            coefficient,
            state: 0.0,
        }
    }

    fn step(&mut self, input: f32) -> f32 {
        self.state += (input - self.state) * self.coefficient;
        self.state
    }
}
