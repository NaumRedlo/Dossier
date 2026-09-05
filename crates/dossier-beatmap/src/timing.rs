#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimingPoint {
    pub time_ms: f64,

    pub beat_length: f64,
    pub meter: u32,

    pub kiai: bool,
}

impl TimingPoint {
    pub fn bpm(&self) -> f64 {
        if self.beat_length > 0.0 {
            60_000.0 / self.beat_length
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleSet {
    #[default]
    Normal,
    Soft,
    Drum,
}

impl SampleSet {
    pub fn from_code(code: u8) -> Self {
        match code {
            2 => Self::Soft,
            3 => Self::Drum,
            _ => Self::Normal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Soft => "soft",
            Self::Drum => "drum",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplePoint {
    pub time_ms: f64,
    pub set: SampleSet,

    pub set_given: bool,

    pub index: u32,

    pub volume: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityPoint {
    pub time_ms: f64,

    pub velocity: f64,
    pub kiai: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timing {
    pub uninherited: Vec<TimingPoint>,
    pub inherited: Vec<VelocityPoint>,

    pub samples: Vec<SamplePoint>,
}

impl Timing {
    pub fn timing_point_at(&self, time_ms: f64) -> Option<&TimingPoint> {
        if self.uninherited.is_empty() {
            return None;
        }
        let idx = self
            .uninherited
            .partition_point(|p| p.time_ms <= time_ms)
            .saturating_sub(1);
        self.uninherited.get(idx).or(self.uninherited.first())
    }

    pub fn velocity_at(&self, time_ms: f64) -> f64 {
        let green = self
            .inherited
            .partition_point(|p| p.time_ms <= time_ms)
            .checked_sub(1)
            .map(|i| &self.inherited[i]);
        let Some(green) = green else {
            return 1.0;
        };
        let red = self
            .uninherited
            .partition_point(|p| p.time_ms <= time_ms)
            .checked_sub(1)
            .map(|i| self.uninherited[i].time_ms);
        match red {
            Some(red) if red > green.time_ms => 1.0,
            _ => green.velocity,
        }
    }

    pub fn bpm_at(&self, time_ms: f64) -> f64 {
        self.timing_point_at(time_ms).map_or(0.0, TimingPoint::bpm)
    }

    pub fn kiai_spans(&self) -> Vec<(f64, f64)> {
        let mut points: Vec<(f64, u8, bool)> = self
            .uninherited
            .iter()
            .map(|p| (p.time_ms, 0u8, p.kiai))
            .chain(self.inherited.iter().map(|p| (p.time_ms, 1u8, p.kiai)))
            .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut spans: Vec<(f64, f64)> = Vec::new();
        let mut open: Option<f64> = None;
        for (time_ms, _, kiai) in points {
            match (open, kiai) {
                (None, true) => open = Some(time_ms),
                (Some(start), false) => {
                    if time_ms > start {
                        spans.push((start, time_ms));
                    }
                    open = None;
                }
                _ => {}
            }
        }
        if let Some(start) = open {
            spans.push((start, f64::INFINITY));
        }
        spans
    }
}

impl Timing {
    pub fn sample_point_at(&self, time_ms: f64) -> Option<&SamplePoint> {
        let index = self.samples.partition_point(|p| p.time_ms <= time_ms);
        if index == 0 {
            self.samples.first()
        } else {
            self.samples.get(index - 1)
        }
    }
}
