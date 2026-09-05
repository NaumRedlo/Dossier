use std::collections::BTreeMap;

use dossier_exhibit::{Clip, Facet, Reason, Settings};
use dossier_replay::Replay;
use dossier_sim::GameState;

use crate::report::quote;

pub fn settings(budget_s: Option<f64>, clip_s: Option<f64>, worth: Option<f64>) -> Settings {
    let defaults = Settings::default();
    Settings {
        budget_ms: budget_s.map_or(defaults.budget_ms, |s| s * 1000.0),
        clip_ms: clip_s.map_or(defaults.clip_ms, |s| s * 1000.0),
        worth: worth.map_or(defaults.worth, |w| w.clamp(0.0, 1.0)),
        ..defaults
    }
}

pub fn as_json(replay_path: &str, replay: &Replay, state: &GameState, clips: &[Clip]) -> String {
    let (from, to) = state.span_ms();
    let clips: Vec<String> = clips
        .iter()
        .map(|clip| {
            format!(
                "{{\"from_ms\":{:.1},\"to_ms\":{:.1},\"rank\":{},\"score\":{:.4},\"scorer\":{},\"reason\":{},\"detail\":{}{}}}",
                clip.span.from_ms,
                clip.span.to_ms,
                clip.rank,
                clip.score,
                quote(clip.reason.scorer().name()),
                quote(&clip.reason.describe()),
                detail(&clip.reason),
                match &clip.with {
                    Some(with) => format!(
                        ",\"with\":{{\"scorer\":{},\"reason\":{},\"detail\":{}}}",
                        quote(with.scorer().name()),
                        quote(&with.describe()),
                        detail(with),
                    ),
                    None => String::new(),
                },
            )
        })
        .collect();
    format!(
        "{{\"replay\":{},\"player\":{},\"rate\":{:.3},\"play_ms\":[{from:.1},{to:.1}],\"clips\":[{}]}}",
        quote(replay_path),
        quote(&replay.player),
        state.playback_rate(),
        clips.join(","),
    )
}

fn detail(reason: &Reason) -> String {
    match *reason {
        Reason::Kiai { bpm, length_ms } => {
            format!("{{\"bpm\":{bpm:.1},\"length_ms\":{length_ms:.0}}}")
        }
        Reason::Peak { combo } => format!("{{\"combo\":{combo}}}"),
        Reason::Choke { combo, through } => {
            format!("{{\"combo\":{combo},\"through\":{through:.4}}}")
        }
        Reason::Storm {
            objects,
            of_densest,
        } => format!("{{\"objects\":{objects},\"of_densest\":{of_densest:.4}}}"),
        Reason::Precision {
            clicks,
            mean_error_ms,
            baseline_ms,
        } => format!(
            "{{\"clicks\":{clicks},\"mean_error_ms\":{mean_error_ms:.2},\"baseline_ms\":{baseline_ms:.2}}}"
        ),
        Reason::Scramble { misses, refused } => {
            format!("{{\"misses\":{misses},\"refused\":{refused}}}")
        }
        Reason::Brink { low, recovered_to } => {
            format!("{{\"low\":{low:.1},\"recovered_to\":{recovered_to:.1}}}")
        }
        Reason::Opening { objects } => format!("{{\"objects\":{objects}}}"),
        Reason::Finale {
            failed,
            accuracy,
            combo,
            full_combo,
        } => format!(
            "{{\"failed\":{failed},\"accuracy\":{accuracy:.4},\"combo\":{combo},\"full_combo\":{full_combo}}}"
        ),
        Reason::Tapping {
            per_second,
            of_hardest,
            taps,
        } => format!(
            "{{\"per_second\":{per_second:.2},\"of_hardest\":{of_hardest:.4},\"taps\":{taps}}}"
        ),
        Reason::Travel { speed, of_fastest } => {
            format!("{{\"speed\":{speed:.1},\"of_fastest\":{of_fastest:.4}}}")
        }
    }
}

pub fn as_text(clips: &[Clip], rate: f64) -> String {
    if clips.is_empty() {
        return "nothing to show — the play is shorter than one clip\n".to_owned();
    }
    let mut out = String::new();
    let mut watched = 0.0;
    for clip in clips {
        let seconds = clip.span.length_ms() / rate / 1000.0;
        watched += seconds;
        out.push_str(&format!(
            "{:>9} {:>9}  {:<10} {}\n",
            stamp(clip.span.from_ms),
            format!("+{seconds:.1}s"),
            clip.reason.scorer().name(),
            clip.reason.describe(),
        ));
        if let Some(with) = &clip.with {
            out.push_str(&format!(
                "{:>9} {:>9}  {:<10} {}\n",
                "",
                "",
                with.scorer().name(),
                with.describe(),
            ));
        }
    }
    out.push_str(&format!(
        "\n{} clip(s), {watched:.1}s to watch\n",
        clips.len()
    ));
    out
}

fn stamp(ms: f64) -> String {
    let total = (ms / 1000.0).max(0.0);
    let minutes = (total / 60.0).floor();
    format!("{minutes:.0}:{:04.1}", total - minutes * 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_read_like_the_editor() {
        assert_eq!(stamp(0.0), "0:00.0");
        assert_eq!(stamp(83_400.0), "1:23.4");
        assert_eq!(stamp(600_000.0), "10:00.0");
    }

    #[test]
    fn a_clip_before_zero_stamps_at_zero() {
        assert_eq!(stamp(-500.0), "0:00.0");
    }
}

#[derive(Default)]
pub struct Survey {
    pub reels: usize,

    pub empty: usize,

    pub skipped: usize,

    lengths: Vec<f64>,

    by_scorer: BTreeMap<&'static str, usize>,

    by_facet: BTreeMap<&'static str, usize>,

    pub no_run: usize,

    pub merged: usize,

    pub map_only: usize,
}

impl Survey {
    pub fn add(&mut self, clips: &[Clip], rate: f64) {
        if clips.is_empty() {
            self.empty += 1;
            return;
        }
        self.reels += 1;
        self.lengths
            .push(clips.iter().map(|c| c.span.length_ms()).sum::<f64>() / 1000.0 / rate.max(0.001));
        for clip in clips {
            for scorer in [Some(clip.reason.scorer()), clip.with.map(|w| w.scorer())]
                .into_iter()
                .flatten()
            {
                *self.by_scorer.entry(scorer.name()).or_insert(0) += 1;
                *self.by_facet.entry(scorer.facet().name()).or_insert(0) += 1;
            }
            if clip.with.is_some() {
                self.merged += 1;
            }
        }
        let facets: Vec<Facet> = clips
            .iter()
            .flat_map(|c| {
                [Some(c.reason.scorer()), c.with.map(|w| w.scorer())]
                    .into_iter()
                    .flatten()
                    .map(|s| s.facet())
                    .collect::<Vec<_>>()
            })
            .collect();
        if !facets.contains(&Facet::Run) {
            self.no_run += 1;
        }
        if facets.iter().all(|facet| *facet == Facet::Map) {
            self.map_only += 1;
        }
    }

    pub fn report(&self) -> String {
        if self.reels == 0 {
            return format!(
                "no reels: {} replay(s) had nothing to show, {} could not be judged\n",
                self.empty, self.skipped
            );
        }
        let mut lengths = self.lengths.clone();
        lengths.sort_by(f64::total_cmp);
        let total: usize = self.by_scorer.values().sum();

        let mut out = format!(
            "{} reel(s) from {} replay(s){}\n{:.0}s…{:.0}s, median {:.0}s, {} clips\n\n",
            self.reels,
            self.reels + self.empty + self.skipped,
            match (self.empty, self.skipped) {
                (0, 0) => String::new(),
                (e, 0) => format!(" — {e} with nothing to show"),
                (0, s) => format!(" — {s} unjudged"),
                (e, s) => format!(" — {e} with nothing to show, {s} unjudged"),
            },
            lengths[0],
            lengths[lengths.len() - 1],
            lengths[lengths.len() / 2],
            total,
        );
        out.push_str(&format!(
            "{:<11}{:>7}{:>10}{:>8}\n",
            "scorer", "clips", "per reel", "share"
        ));

        let mut rows: Vec<(&str, usize)> = self.by_scorer.iter().map(|(k, v)| (*k, *v)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        for (name, count) in rows {
            out.push_str(&format!(
                "{name:<11}{count:>7}{:>10.2}{:>7.0}%\n",
                count as f64 / self.reels as f64,
                count as f64 / total as f64 * 100.0,
            ));
        }
        out.push_str("\nby facet\n");
        for facet in [Facet::Run, Facet::Hand, Facet::Map] {
            let count = self.by_facet.get(facet.name()).copied().unwrap_or(0);
            out.push_str(&format!(
                "{:<11}{count:>7}{:>10.2}{:>7.0}%\n",
                facet.name(),
                count as f64 / self.reels as f64,
                count as f64 / total as f64 * 100.0,
            ));
        }
        if self.merged > 0 {
            out.push_str(&format!("\n{} clip(s) hold two moments\n", self.merged));
        }
        out.push_str(&format!(
            "\nreels with nothing about the run: {} of {} ({:.0}%)\n",
            self.no_run,
            self.reels,
            self.no_run as f64 / self.reels as f64 * 100.0,
        ));
        if self.map_only > 0 {
            out.push_str(&format!(
                "reels about the map alone:        {} ({:.0}%)\n",
                self.map_only,
                self.map_only as f64 / self.reels as f64 * 100.0,
            ));
        }
        out
    }
}

#[cfg(test)]
mod survey_tests {
    use super::*;
    use dossier_exhibit::Span;

    fn clip(scorer_reason: Reason, from: f64, to: f64) -> Clip {
        Clip {
            span: Span::new(from, to),
            reason: scorer_reason,
            with: None,
            rank: 0,
            score: 0.5,
        }
    }

    #[test]
    fn a_reel_with_nothing_about_the_run_is_counted() {
        let mut survey = Survey::default();

        survey.add(
            &[
                clip(
                    Reason::Storm {
                        objects: 60,
                        of_densest: 1.0,
                    },
                    0.0,
                    6000.0,
                ),
                clip(
                    Reason::Travel {
                        speed: 500.0,
                        of_fastest: 1.0,
                    },
                    9000.0,
                    15000.0,
                ),
            ],
            1.0,
        );

        survey.add(
            &[
                clip(
                    Reason::Storm {
                        objects: 60,
                        of_densest: 1.0,
                    },
                    0.0,
                    6000.0,
                ),
                clip(
                    Reason::Choke {
                        combo: 900,
                        through: 0.8,
                    },
                    9000.0,
                    15000.0,
                ),
            ],
            1.0,
        );
        assert_eq!((survey.reels, survey.no_run, survey.map_only), (2, 1, 0));

        let report = survey.report();
        assert!(
            report.contains("nothing about the run: 1 of 2 (50%)"),
            "{report}"
        );
    }

    #[test]
    fn an_empty_selection_is_not_a_reel() {
        let mut survey = Survey::default();
        survey.add(&[], 1.0);
        assert_eq!((survey.reels, survey.empty), (0, 1));
        assert!(
            survey.report().starts_with("no reels:"),
            "{}",
            survey.report()
        );
    }

    #[test]
    fn lengths_are_seconds_of_watching() {
        let mut survey = Survey::default();
        survey.add(&[clip(Reason::Peak { combo: 500 }, 0.0, 9000.0)], 1.5);
        assert!(survey.report().contains("6s…6s"), "{}", survey.report());
    }
}

#[cfg(test)]
mod merged_survey_tests {
    use super::*;
    use dossier_exhibit::Span;

    #[test]
    fn both_moments_of_a_merged_clip_are_counted() {
        let mut survey = Survey::default();
        survey.add(
            &[Clip {
                span: Span::new(0.0, 12_000.0),
                reason: Reason::Scramble {
                    misses: 42,
                    refused: 33,
                },
                with: Some(Reason::Travel {
                    speed: 964.0,
                    of_fastest: 1.0,
                }),
                rank: 0,
                score: 0.8,
            }],
            1.0,
        );
        assert_eq!(survey.merged, 1);
        let report = survey.report();
        assert!(report.contains("scramble"), "{report}");
        assert!(report.contains("travel"), "{report}");
        assert!(report.contains("hold two moments"), "{report}");

        assert!(report.contains("run"), "{report}");
        assert!(report.contains("hand"), "{report}");
    }
}
