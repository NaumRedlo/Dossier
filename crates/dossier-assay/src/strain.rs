#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrainPeak {
    pub value: f64,

    pub section_length: f64,
}

impl StrainPeak {
    pub fn new(value: f64, section_length: f64) -> Self {
        Self {
            value,
            section_length: section_length.round(),
        }
    }
}

fn max_stored_length(decay_weight: f64) -> f64 {
    11.0 / (1.0 - decay_weight)
}

pub struct Sections {
    pub decay_weight: f64,
    pub max_section_length: f64,
    peak: f64,
    begin: f64,
    end: f64,

    peaks: Vec<StrainPeak>,
    total_length: f64,

    queued: Vec<(f64, f64)>,

    final_peak: Option<StrainPeak>,
}

impl Sections {
    pub fn new(decay_weight: f64, max_section_length: f64) -> Self {
        Self {
            decay_weight,
            max_section_length,
            peak: 0.0,
            begin: 0.0,
            end: 0.0,
            peaks: Vec::new(),
            total_length: 0.0,
            queued: Vec::new(),
            final_peak: None,
        }
    }

    pub fn begin_at(&mut self, start_time: f64, strain: f64) {
        self.begin = start_time;
        self.end = start_time + self.max_section_length;
        self.peak = strain;
    }

    pub fn take(&mut self, start_time: f64, strain: f64, initial_strain: &dyn Fn(f64) -> f64) {
        self.backfill(start_time, initial_strain);

        if strain > self.peak {
            self.queued.clear();
            self.save(start_time - self.begin);
            self.begin = start_time;
            self.end = start_time + self.max_section_length;
            self.peak = strain;
        } else {
            while self.queued.last().is_some_and(|(value, _)| *value < strain) {
                self.queued.pop();
            }
            self.queued.push((strain, start_time));
        }
    }

    fn backfill(&mut self, start_time: f64, initial_strain: &dyn Fn(f64) -> f64) {
        while start_time > self.end {
            self.save(self.end - self.begin);
            self.begin = self.end;

            if self.queued.is_empty() {
                self.end = self.begin + self.max_section_length;
                self.peak = initial_strain(self.begin);
            } else {
                let (strain, at) = self.queued.remove(0);

                self.end = at + self.max_section_length;
                self.peak = initial_strain(self.begin);
                self.peak = self.peak.max(strain);
            }
        }
    }

    fn save(&mut self, section_length: f64) {
        if let Some(open) = self.final_peak.take() {
            if let Some(at) = self.peaks.iter().position(|peak| *peak == open) {
                self.peaks.remove(at);
            }
        }

        let peak = StrainPeak::new(self.peak, section_length);
        let at = self.peaks.partition_point(|other| other.value > peak.value);
        self.peaks.insert(at, peak);
        self.total_length += peak.section_length;

        while self.total_length > max_stored_length(self.decay_weight) * self.max_section_length {
            if let Some(dropped) = self.peaks.pop() {
                self.total_length -= dropped.section_length;
            } else {
                break;
            }
        }
    }

    pub fn peaks(&mut self) -> &[StrainPeak] {
        if self.final_peak.is_none() {
            let peak = StrainPeak::new(self.peak, self.end - self.begin);
            let at = self.peaks.partition_point(|other| other.value > peak.value);
            self.peaks.insert(at, peak);
            self.final_peak = Some(peak);
        }
        &self.peaks
    }
}

fn lerp(from: f64, to: f64, at: f64) -> f64 {
    from + (to - from) * at
}

pub fn reduced_peaks(peaks: &[StrainPeak]) -> Vec<StrainPeak> {
    const REDUCED_SECTION_TIME: f64 = 4000.0;
    const REDUCED_STRAIN_BASELINE: f64 = 0.727;
    const CHUNK_SIZE: f64 = 20.0;

    let mut strains: Vec<StrainPeak> = peaks
        .iter()
        .copied()
        .filter(|peak| peak.value > 0.0)
        .collect();

    let mut time = 0.0;
    let mut skip = 0usize;
    while strains.len() > skip && time < REDUCED_SECTION_TIME {
        let strain = strains[skip];
        let mut added = 0.0;
        while added < strain.section_length {
            let scale = lerp(
                1.0,
                10.0,
                ((time + added) / REDUCED_SECTION_TIME).clamp(0.0, 1.0),
            )
            .log10();

            strains.push(StrainPeak::new(
                strain.value * lerp(REDUCED_STRAIN_BASELINE, 1.0, scale),
                CHUNK_SIZE.min(strain.section_length - added),
            ));
            added += CHUNK_SIZE;
        }
        time += strain.section_length;
        skip += 1;
    }

    let mut out: Vec<StrainPeak> = strains.split_off(skip.min(strains.len()));
    out.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

pub fn difficulty_value(peaks: &[StrainPeak], decay_weight: f64, max_section_length: f64) -> f64 {
    let mut difficulty = 0.0;
    let mut time = 0.0;
    for peak in peaks {
        let start = time;
        let end = time + peak.section_length / max_section_length;
        difficulty += peak.value * (decay_weight.powf(start) - decay_weight.powf(end));
        time = end;
    }
    difficulty / (1.0 - decay_weight)
}
