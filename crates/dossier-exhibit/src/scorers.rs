use dossier_sim::{GameState, Part};

use crate::{Candidate, Reason, Settings, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum Facet {
    Map,

    Hand,

    Run,
}

impl Facet {
    pub fn name(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Hand => "hand",
            Self::Run => "run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scorer {
    Kiai,
    Brink,
    Tapping,
    Peak,
    Choke,
    Storm,
    Precision,
    Scramble,
    Opening,
    Finale,
    Travel,
}

impl Scorer {
    pub fn name(self) -> &'static str {
        match self {
            Self::Kiai => "kiai",
            Self::Brink => "brink",
            Self::Tapping => "tapping",
            Self::Peak => "peak",
            Self::Choke => "choke",
            Self::Storm => "storm",
            Self::Precision => "precision",
            Self::Scramble => "scramble",
            Self::Opening => "opening",
            Self::Finale => "finale",
            Self::Travel => "travel",
        }
    }

    pub fn facet(self) -> Facet {
        match self {
            Self::Kiai | Self::Storm | Self::Opening => Facet::Map,
            Self::Travel | Self::Precision | Self::Tapping => Facet::Hand,
            Self::Choke | Self::Peak | Self::Scramble | Self::Finale | Self::Brink => Facet::Run,
        }
    }

    pub fn can_repeat(self) -> bool {
        !matches!(self, Self::Opening | Self::Finale)
    }

    pub(crate) fn weight(self) -> f64 {
        match self {
            Self::Choke => 1.00,

            Self::Brink => 0.97,

            Self::Finale => 0.95,
            Self::Peak => 0.90,
            Self::Scramble => 0.80,
            Self::Precision => 0.70,

            Self::Travel => 0.65,

            Self::Tapping => 0.62,
            Self::Kiai => 0.60,
            Self::Storm => 0.50,

            Self::Opening => 0.45,
        }
    }
}

fn notable(x: f64, half: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    let k = half * half;
    (x * x * (1.0 + k)) / (x * x + k)
}

const RUN_HALF: f64 = 0.35;

const TROUBLE_HALF: f64 = 0.06;

const SCRAMBLE_MIN: usize = 3;

pub(crate) fn all(state: &GameState, settings: Settings) -> Vec<(Scorer, Candidate)> {
    let mut out = Vec::new();
    for (scorer, found) in [
        (Scorer::Kiai, kiai(state, settings)),
        (Scorer::Brink, brink(state)),
        (Scorer::Peak, peak(state)),
        (Scorer::Choke, choke(state)),
        (Scorer::Storm, storm(state, settings)),
        (Scorer::Precision, precision(state, settings)),
        (Scorer::Scramble, scramble(state, settings)),
        (Scorer::Opening, opening(state, settings)),
        (Scorer::Finale, finale(state)),
        (Scorer::Travel, travel(state, settings)),
        (Scorer::Tapping, tapping(state, settings)),
    ] {
        out.extend(found.into_iter().map(|c| (scorer, c)));
    }

    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.anchor_ms.total_cmp(&b.1.anchor_ms)));
    out
}

fn kiai(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let timeline = state.timeline();
    let (play_from, play_to) = state.span_ms();
    let mut out = Vec::new();
    for (start, end) in timeline.timing.kiai_spans() {
        let from = start.max(play_from);
        let to = end.min(play_to);

        if to - from < settings.clip_ms / 2.0 {
            continue;
        }
        let bpm = timeline.timing.bpm_at(start) * state.playback_rate();
        out.push(Candidate {
            anchor_ms: from,

            bias: 0.12,

            strength: ((to - from) / (settings.clip_ms * 2.0)).min(1.0),
            reason: Reason::Kiai {
                bpm,
                length_ms: to - from,
            },
        });
    }
    out
}

const RUNS_PROPOSED: usize = 3;

fn peak(state: &GameState) -> Vec<Candidate> {
    let full_combo = f64::from(state.max_possible_combo()).max(1.0);
    let last_object_ms = state
        .timeline()
        .objects
        .last()
        .map_or(0.0, |object| object.end_ms);
    state
        .combo_chains()
        .into_iter()
        .take(RUNS_PROPOSED)
        .filter(|chain| chain.length > 0)
        .map(|chain| Candidate {
            anchor_ms: if chain.ended_at_ms.is_finite() {
                chain.ended_at_ms
            } else {
                last_object_ms
            },
            bias: 1.0,
            strength: notable(f64::from(chain.length) / full_combo, RUN_HALF),
            reason: Reason::Peak {
                combo: chain.length,
            },
        })
        .collect()
}

fn choke(state: &GameState) -> Vec<Candidate> {
    let (play_from, play_to) = state.span_ms();
    let played = (play_to - play_from).max(1.0);

    let full_combo = f64::from(state.max_possible_combo()).max(1.0);
    state
        .combo_chains()
        .into_iter()
        .filter(|chain| chain.part.is_some() && chain.ended_at_ms.is_finite() && chain.length > 0)
        .take(RUNS_PROPOSED)
        .map(|chain| {
            let through = ((chain.ended_at_ms - play_from) / played).clamp(0.0, 1.0);
            Candidate {
                anchor_ms: chain.ended_at_ms,

                bias: 0.7,

                strength: (notable(f64::from(chain.length) / full_combo, RUN_HALF)
                    * (0.5 + through))
                    .min(1.0),
                reason: Reason::Choke {
                    combo: chain.length,
                    through,
                },
            }
        })
        .collect()
}

const PRECISION_MIN_CLICKS: usize = 10;

const PRECISION_HALF: f64 = 0.35;

fn precision(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let Some(judge) = state.judge() else {
        return Vec::new();
    };
    let clicks: Vec<(f64, f64)> = judge.errors_ms().map(|(at, err)| (at, err.abs())).collect();
    if clicks.len() < PRECISION_MIN_CLICKS {
        return Vec::new();
    }
    let baseline = clicks.iter().map(|(_, err)| err).sum::<f64>() / clicks.len() as f64;

    let mut windows = Vec::with_capacity(clicks.len());
    let mut end = 0usize;
    let mut sum = 0.0;
    for start in 0..clicks.len() {
        if end < start {
            end = start;
            sum = 0.0;
        }
        while end < clicks.len() && clicks[end].0 < clicks[start].0 + settings.clip_ms {
            sum += clicks[end].1;
            end += 1;
        }
        let count = end - start;
        let strength = if count >= PRECISION_MIN_CLICKS {
            let mean = sum / count as f64;

            notable(
                ((baseline - mean) / baseline).clamp(0.0, 1.0),
                PRECISION_HALF,
            )
        } else {
            0.0
        };
        windows.push((clicks[start].0, strength, count, sum));
        sum -= clicks[start].1;
    }

    peaks(&windows, |w| w.1)
        .into_iter()
        .map(|i| {
            let (at, strength, count, sum) = windows[i];
            Candidate {
                anchor_ms: at,

                bias: 0.0,
                strength,
                reason: Reason::Precision {
                    clicks: count,
                    mean_error_ms: sum / count as f64,
                    baseline_ms: baseline,
                },
            }
        })
        .collect()
}

const REFUSAL_WEIGHT: f64 = 0.4;

fn scramble(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let Some(judge) = state.judge() else {
        return Vec::new();
    };

    let mut trouble: Vec<(f64, bool)> = judge
        .events()
        .iter()
        .filter(|event| {
            event.result.is_miss()
                && matches!(event.part, Part::Circle | Part::SliderHead | Part::Spinner)
        })
        .map(|event| (event.time_ms, true))
        .chain(judge.shakes().iter().map(|&(_, at)| (at, false)))
        .collect();
    trouble.sort_by(|a, b| a.0.total_cmp(&b.0));
    if trouble.is_empty() {
        return Vec::new();
    }

    let starts: Vec<f64> = state
        .timeline()
        .objects
        .iter()
        .map(|object| object.start_ms)
        .collect();
    let objects_in = |from: f64| -> usize {
        let to = from + settings.clip_ms;
        starts.partition_point(|&at| at < to) - starts.partition_point(|&at| at < from)
    };

    let mut windows = Vec::with_capacity(trouble.len());
    let mut end = 0usize;
    let (mut misses, mut refused) = (0usize, 0usize);
    for start in 0..trouble.len() {
        if end < start {
            end = start;
            misses = 0;
            refused = 0;
        }
        while end < trouble.len() && trouble[end].0 < trouble[start].0 + settings.clip_ms {
            if trouble[end].1 {
                misses += 1;
            } else {
                refused += 1;
            }
            end += 1;
        }

        let trouble_here = misses as f64 + refused as f64 * REFUSAL_WEIGHT;
        let strength = if misses + refused >= SCRAMBLE_MIN {
            notable(
                trouble_here / objects_in(trouble[start].0).max(1) as f64,
                TROUBLE_HALF,
            )
        } else {
            0.0
        };
        windows.push((trouble[start].0, strength, misses, refused));
        if trouble[start].1 {
            misses -= 1;
        } else {
            refused -= 1;
        }
    }

    peaks(&windows, |w| w.1)
        .into_iter()
        .map(|i| {
            let (at, strength, misses, refused) = windows[i];
            Candidate {
                anchor_ms: at,

                bias: 0.25,
                strength,
                reason: Reason::Scramble { misses, refused },
            }
        })
        .collect()
}

fn tapping(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let spinners = outside_spinners(state);
    let in_spinner = |at: f64| spinners.iter().any(|&(from, to)| at >= from && at <= to);

    let mut presses: Vec<f64> = state
        .cursor_track()
        .holds()
        .iter()
        .flat_map(|button| button.iter().map(|&(from, _)| from))
        .filter(|at| !in_spinner(*at))
        .collect();
    presses.sort_by(f64::total_cmp);
    if presses.len() < 2 {
        return Vec::new();
    }

    let mut windows = Vec::with_capacity(presses.len());
    let mut end = 0usize;
    for start in 0..presses.len() {
        end = end.max(start);
        while end < presses.len() && presses[end] < presses[start] + settings.clip_ms {
            end += 1;
        }
        windows.push((presses[start], (end - start) as f64, end - start));
    }

    let hardest = windows.iter().map(|w| w.1).fold(0.0f64, f64::max);
    if !hardest.is_finite() || hardest <= 0.0 {
        return Vec::new();
    }

    peaks(&windows, |w| w.1)
        .into_iter()
        .map(|i| {
            let (at, count, taps) = windows[i];
            Candidate {
                anchor_ms: at,
                bias: 0.0,
                strength: count / hardest,
                reason: Reason::Tapping {
                    per_second: count / (settings.clip_ms / 1000.0) * state.playback_rate(),
                    of_hardest: count / hardest,
                    taps,
                },
            }
        })
        .collect()
}

const SLIDER_DENSITY: f64 = 1.4;

const SPINNER_DENSITY: f64 = 0.2;

fn density_curve(state: &GameState, settings: Settings) -> Vec<(f64, f64, usize)> {
    let objects = &state.timeline().objects;
    if objects.is_empty() {
        return Vec::new();
    }
    let density = |object: &dossier_sim::TimedObject| {
        if object.is_spinner() {
            SPINNER_DENSITY
        } else if matches!(object.kind, dossier_sim::TimedKind::Slider { .. }) {
            SLIDER_DENSITY
        } else {
            1.0
        }
    };

    let mut windows = Vec::with_capacity(objects.len());
    let mut end = 0usize;
    let mut sum = 0.0;
    for start in 0..objects.len() {
        if end < start {
            end = start;
            sum = 0.0;
        }
        while end < objects.len()
            && objects[end].start_ms < objects[start].start_ms + settings.clip_ms
        {
            sum += density(&objects[end]);
            end += 1;
        }
        windows.push((objects[start].start_ms, sum, end - start));
        sum -= density(&objects[start]);
    }

    let densest = windows.iter().map(|w| w.1).fold(0.0f64, f64::max).max(1.0);
    for window in &mut windows {
        window.1 /= densest;
    }
    windows
}

fn storm(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let windows = density_curve(state, settings);
    if windows.is_empty() {
        return Vec::new();
    }

    peaks(&windows, |w| w.1)
        .into_iter()
        .map(|i| {
            let (at, strength, count) = windows[i];
            Candidate {
                anchor_ms: at,
                bias: 0.0,
                strength,
                reason: Reason::Storm {
                    objects: count,
                    of_densest: strength,
                },
            }
        })
        .collect()
}

use dossier_sim::DANGER_LEVEL as BRINK_LEVEL;

const BRINK_STEP_MS: f64 = 100.0;

fn brink(state: &GameState) -> Vec<Candidate> {
    if state.mods().contains(dossier_replay::bits::NO_FAIL) {
        return Vec::new();
    }
    let (from, to) = state.span_ms();
    if state.health_at(from).is_none() || to <= from {
        return Vec::new();
    }

    let mut samples = Vec::with_capacity(((to - from) / BRINK_STEP_MS) as usize + 2);
    let mut at = from;
    while at <= to {
        samples.push((at, state.health_at(at).unwrap_or(1.0)));
        at += BRINK_STEP_MS;
    }

    let mut out = Vec::new();
    let mut index = 0usize;
    while index < samples.len() {
        if samples[index].1 > BRINK_LEVEL {
            index += 1;
            continue;
        }

        let start = index;
        let mut lowest = index;
        while index < samples.len() && samples[index].1 <= BRINK_LEVEL {
            if samples[index].1 < samples[lowest].1 {
                lowest = index;
            }
            index += 1;
        }
        let _ = start;

        if index >= samples.len() {
            break;
        }
        let low = f64::from(samples[lowest].1);
        out.push(Candidate {
            anchor_ms: samples[lowest].0,

            bias: 0.6,

            strength: ((f64::from(BRINK_LEVEL) - low) / f64::from(BRINK_LEVEL)).clamp(0.0, 1.0),
            reason: Reason::Brink {
                low: low * 100.0,
                recovered_to: f64::from(samples[index].1) * 100.0,
            },
        });
    }
    out
}

fn opening(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let (play_from, play_to) = state.span_ms();
    if play_to - play_from < settings.clip_ms {
        return Vec::new();
    }
    let curve = density_curve(state, settings);

    let Some(&(_, share, objects)) = curve.first() else {
        return Vec::new();
    };
    vec![Candidate {
        anchor_ms: play_from,
        bias: 0.0,

        strength: share * share,
        reason: Reason::Opening { objects },
    }]
}

fn finale(state: &GameState) -> Vec<Candidate> {
    let (play_from, play_to) = state.span_ms();
    let Some(judge) = state.judge() else {
        return Vec::new();
    };
    let failed = state.ending().is_some();
    let final_state = match state.ending() {
        Some(end) => end.score,
        None => judge.final_state(),
    };
    let accuracy = final_state.accuracy();
    let full_combo =
        final_state.max_combo >= state.max_possible_combo() && state.max_possible_combo() > 0;

    let strength = if failed || full_combo {
        1.0
    } else {
        ((accuracy - 90.0) / 10.0).clamp(0.0, 1.0)
    };
    if !strength.is_finite() || strength <= 0.0 || play_to <= play_from {
        return Vec::new();
    }
    vec![Candidate {
        anchor_ms: play_to,

        bias: 1.0,
        strength,
        reason: Reason::Finale {
            failed,
            accuracy,
            combo: final_state.max_combo,
            full_combo,
        },
    }]
}

fn outside_spinners(state: &GameState) -> Vec<(f64, f64)> {
    state
        .timeline()
        .objects
        .iter()
        .filter(|object| object.is_spinner())
        .map(|object| (object.start_ms, object.end_ms))
        .collect()
}

fn travel(state: &GameState, settings: Settings) -> Vec<Candidate> {
    let frames = state.cursor_track().frames();
    if frames.len() < 2 {
        return Vec::new();
    }
    let spinners = outside_spinners(state);
    let in_spinner = |at: f64| spinners.iter().any(|&(from, to)| at >= from && at <= to);

    let steps: Vec<(f64, f64)> = frames
        .windows(2)
        .filter_map(|pair| {
            let (a, b) = (&pair[0], &pair[1]);
            let at = b.time_ms as f64;
            if in_spinner(at) {
                return None;
            }
            let (dx, dy) = (f64::from(b.x - a.x), f64::from(b.y - a.y));
            Some((at, (dx * dx + dy * dy).sqrt()))
        })
        .collect();
    if steps.is_empty() {
        return Vec::new();
    }

    let mut windows = Vec::with_capacity(steps.len());
    let mut end = 0usize;
    let mut sum = 0.0;
    for start in 0..steps.len() {
        if end < start {
            end = start;
            sum = 0.0;
        }
        while end < steps.len() && steps[end].0 < steps[start].0 + settings.clip_ms {
            sum += steps[end].1;
            end += 1;
        }
        windows.push((steps[start].0, sum, 0usize));
        sum -= steps[start].1;
    }

    let fastest = windows.iter().map(|w| w.1).fold(0.0f64, f64::max);
    if !fastest.is_finite() || fastest <= 0.0 {
        return Vec::new();
    }

    peaks(&windows, |w| w.1)
        .into_iter()
        .map(|i| {
            let (at, distance, _) = windows[i];
            Candidate {
                anchor_ms: at,
                bias: 0.0,
                strength: distance / fastest,
                reason: Reason::Travel {
                    speed: distance / (settings.clip_ms / 1000.0),
                    of_fastest: distance / fastest,
                },
            }
        })
        .collect()
}

fn peaks<T>(windows: &[T], weight: impl Fn(&T) -> f64) -> Vec<usize> {
    (0..windows.len())
        .filter(|&i| {
            let here = weight(&windows[i]);
            here > 0.0
                && (i == 0 || here > weight(&windows[i - 1]))
                && (i + 1 == windows.len() || here >= weight(&windows[i + 1]))
        })
        .collect()
}

pub(crate) fn clip_for(candidate: &Candidate, clip_ms: f64, play: (f64, f64)) -> Span {
    let bias = candidate.bias.clamp(0.0, 1.0);
    let span = Span::new(
        candidate.anchor_ms - bias * clip_ms,
        candidate.anchor_ms + (1.0 - bias) * clip_ms,
    );
    if span.from_ms < play.0 {
        span.shifted_to(play.0)
    } else if span.to_ms > play.1 {
        span.shifted_to((play.1 - clip_ms).max(play.0))
    } else {
        span
    }
}
