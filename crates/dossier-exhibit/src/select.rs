use dossier_beatmap::Timing;
use dossier_sim::Timeline;

use crate::scorers::{clip_for, Scorer};
use crate::{Candidate, Clip, Settings, Span};

const SNAP_LIMIT: f64 = 0.5;

const SNAP_SHARE: f64 = 0.1;

pub(crate) const REPEAT_DECAY: f64 = 0.55;

const FACET_DECAY: f64 = 0.75;

const CROWDED: f64 = 0.25;

const REPEAT_REACH: f64 = 5.0;

const MOST_OF_A_PLAY: f64 = 0.4;

const LEAST_REEL_CLIPS: f64 = 3.0;

const MERGE_APART: f64 = 1.0 / 3.0;

const MERGE_ROOM: f64 = 1.0;

pub(crate) fn choose(
    candidates: Vec<(Scorer, Candidate)>,
    play: (f64, f64),
    timeline: &Timeline,
    settings: Settings,
) -> Vec<Clip> {
    if candidates.is_empty()
        || settings.clip_ms <= 0.0
        || settings.budget_ms < settings.clip_ms
        || play.1 - play.0 < settings.clip_ms
    {
        return Vec::new();
    }

    let ranked = rank(candidates, settings, play);

    let spread_ms = settings.spread * settings.clip_ms;

    let reach = REPEAT_REACH * settings.clip_ms;
    let mut chosen: Vec<Chosen> = Vec::new();
    let mut spent = vec![false; ranked.len()];

    let mut budget_left = settings
        .budget_ms
        .min(((play.1 - play.0) * MOST_OF_A_PLAY).max(LEAST_REEL_CLIPS * settings.clip_ms));
    loop {
        let mut best: Option<Pick> = None;
        for (index, candidate) in ranked.iter().enumerate() {
            if spent[index] || candidate.span.length_ms() > budget_left {
                continue;
            }

            let merge = match merge_into(&chosen, candidate, settings) {
                Merge::Fresh => None,
                Merge::Into(into, span) => Some((into, span)),
                Merge::No => continue,
            };
            let cost = match merge {
                Some((into, span)) => span.length_ms() - chosen[into].span.length_ms(),
                None => candidate.span.length_ms(),
            };
            if cost > budget_left {
                continue;
            }

            let crowded = merge.is_none()
                && chosen
                    .iter()
                    .any(|already| (already.anchor_ms - candidate.anchor_ms).abs() < spread_ms);

            let (repeats, facet_repeats) = if candidate.scorer.can_repeat() {
                chosen.iter().fold((0.0, 0), |(same, kind), already| {
                    let near = (1.0 - (already.anchor_ms - candidate.anchor_ms).abs() / reach)
                        .clamp(0.0, 1.0);
                    (
                        same + if already.scorer == candidate.scorer {
                            near
                        } else {
                            0.0
                        },
                        kind + i32::from(already.scorer.facet() == candidate.scorer.facet()),
                    )
                })
            } else {
                (0.0, 0)
            };
            let effective = candidate.score
                * REPEAT_DECAY.powf(repeats)
                * FACET_DECAY.powi(facet_repeats)
                * if crowded { CROWDED } else { 1.0 };
            if best.is_none_or(|top| effective > top.score) {
                best = Some(Pick {
                    score: effective,
                    index,
                    merge,
                    cost,
                });
            }
        }

        let Some(Pick {
            score: effective,
            index,
            merge,
            cost,
        }) = best.filter(|pick| pick.score >= settings.worth)
        else {
            break;
        };
        spent[index] = true;
        budget_left -= cost;
        match merge {
            Some((into, span)) => {
                chosen[into].span = span;
                chosen[into].with = Some(ranked[index].reason);
            }
            None => {
                let mut clip = ranked[index];
                clip.score = effective;
                chosen.push(clip);
            }
        }
    }

    for (rank, clip) in chosen.iter_mut().enumerate() {
        clip.rank = rank;
    }
    chosen.sort_by(|a, b| a.span.from_ms.total_cmp(&b.span.from_ms));

    chosen
        .into_iter()
        .map(|clip| Clip {
            span: snap(clip.span, &timeline.timing, play),
            reason: clip.reason,
            with: clip.with,
            rank: clip.rank,
            score: clip.score,
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct Chosen {
    span: Span,
    anchor_ms: f64,
    scorer: Scorer,
    score: f64,
    rank: usize,
    reason: crate::Reason,
    with: Option<crate::Reason>,
}

#[derive(Clone, Copy)]
struct Pick {
    score: f64,
    index: usize,

    merge: Option<(usize, Span)>,

    cost: f64,
}

enum Merge {
    Fresh,

    Into(usize, Span),

    No,
}

fn merge_into(chosen: &[Chosen], candidate: &Chosen, settings: Settings) -> Merge {
    let mut found = None;
    for (index, already) in chosen.iter().enumerate() {
        if !already.span.overlaps(&candidate.span) {
            continue;
        }
        if found.is_some() {
            return Merge::No;
        }
        found = Some(index);
    }
    let Some(index) = found else {
        return Merge::Fresh;
    };
    let into = &chosen[index];
    if into.with.is_some() || into.scorer == candidate.scorer {
        return Merge::No;
    }
    if (into.anchor_ms - candidate.anchor_ms).abs() < settings.clip_ms * MERGE_APART {
        return Merge::No;
    }

    let span = Span::new(
        into.span.from_ms.min(candidate.span.from_ms),
        into.span.to_ms.max(candidate.span.to_ms),
    );
    let longest = settings.length_for(1.0) + settings.clip_ms * MERGE_ROOM;
    if span.length_ms() > longest {
        return Merge::No;
    }
    Merge::Into(index, span)
}

fn rank(candidates: Vec<(Scorer, Candidate)>, settings: Settings, play: (f64, f64)) -> Vec<Chosen> {
    let mut ranked: Vec<Chosen> = candidates
        .iter()
        .filter_map(|(scorer, candidate)| {
            let strength = candidate.strength.clamp(0.0, 1.0);
            if !strength.is_finite() || strength <= 0.0 {
                return None;
            }

            let score = strength * scorer.weight();
            let length = settings.length_for(score).min(play.1 - play.0);
            Some(Chosen {
                span: clip_for(candidate, length, play),
                anchor_ms: candidate.anchor_ms,
                scorer: *scorer,
                score,
                rank: 0,
                reason: candidate.reason,
                with: None,
            })
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.anchor_ms.total_cmp(&b.anchor_ms))
    });
    ranked
}

fn snap(span: Span, timing: &Timing, play: (f64, f64)) -> Span {
    if (span.from_ms - play.0).abs() < 1.0 || (span.to_ms - play.1).abs() < 1.0 {
        return span;
    }
    let Some(point) = timing.timing_point_at(span.from_ms) else {
        return span;
    };
    let beat = point.beat_length;
    if !beat.is_finite() || beat <= 0.0 {
        return span;
    }

    let bar = beat * f64::from(point.meter.max(1));
    let allowance = span.length_ms() * SNAP_SHARE;
    let snapped = [bar, beat].into_iter().find_map(|unit| {
        let steps = (span.from_ms - point.time_ms) / unit;
        let at = point.time_ms + steps.round() * unit;
        let moved = (at - span.from_ms).abs();
        (moved <= unit * SNAP_LIMIT && moved <= allowance).then_some(at)
    });
    let Some(snapped) = snapped else {
        return span;
    };
    let moved = span.shifted_to(snapped);

    if moved.from_ms < play.0 || moved.to_ms > play.1 {
        span
    } else {
        moved
    }
}
