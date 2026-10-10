use dossier_assay::performance::{performance, Score};
use dossier_assay::{attributes, Attributes};
use dossier_beatmap::Beatmap;
use dossier_replay::Mods;

use crate::stable::Counts;

pub struct Pace {
    map: Beatmap,
    mods: Mods,
    whole: Attributes,
    part: Option<(usize, Attributes)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    pub now: f64,
    pub clean: f64,
}

impl Pace {
    pub fn of(text: &str, mods: u32) -> Option<Pace> {
        let map = Beatmap::parse(text).ok().filter(|map| map.mode == 0 && !map.objects.is_empty())?;
        let mods = Mods::new(mods);
        let whole = attributes(&map, mods);
        Some(Pace { map, mods, whole, part: None })
    }

    pub fn objects(&self) -> usize {
        self.map.objects.len()
    }

    pub fn judged(&self, counts: Counts) -> usize {
        (usize::from(counts.n300) + usize::from(counts.n100) + usize::from(counts.n50) + usize::from(counts.miss)).min(self.objects())
    }

    fn passed(&mut self, judged: usize) -> &Attributes {
        if self.part.as_ref().is_none_or(|(passed, _)| *passed != judged) {
            let mut part = self.map.clone();
            part.objects.truncate(judged);
            self.part = Some((judged, attributes(&part, self.mods)));
        }
        &self.part.as_ref().expect("the passed part was just measured").1
    }

    pub fn best(text: &str, scores: &[crate::scores::Held]) -> Option<f64> {
        let map = Beatmap::parse(text).ok().filter(|map| map.mode == 0 && !map.objects.is_empty())?;
        let mut measured: Vec<(u32, Attributes)> = Vec::new();
        let mut best = None::<f64>;
        for held in scores {
            let mods = Mods::new(held.mods);
            if !measured.iter().any(|(known, _)| *known == held.mods) {
                measured.push((held.mods, attributes(&map, mods)));
            }
            let whole = &measured.iter().find(|(known, _)| *known == held.mods)?.1;
            let mut score = Score {
                max_combo: u32::from(held.max_combo).min(whole.max_combo),
                great: u32::from(held.counts.n300),
                ok: u32::from(held.counts.n100),
                meh: u32::from(held.counts.n50),
                miss: u32::from(held.counts.miss),
                slider_tail_hit: whole.slider_count,
                large_tick_miss: 0,
                classic: true,
                legacy_total_score: None,
                accuracy: None,
            };
            score.accuracy = Some(score.accuracy());
            let pp = performance(&score, whole, mods).pp;
            if pp.is_finite() && pp > 0.0 {
                best = Some(best.map_or(pp, |known| known.max(pp)));
            }
        }
        best
    }

    pub fn clean(&self, counts: Counts) -> f64 {
        let objects = self.objects() as u32;
        let (ok, meh) = (u32::from(counts.n100).min(objects), u32::from(counts.n50).min(objects));
        let mut score = Score {
            max_combo: self.whole.max_combo,
            great: objects.saturating_sub(ok).saturating_sub(meh),
            ok,
            meh,
            miss: 0,
            slider_tail_hit: self.whole.slider_count,
            large_tick_miss: 0,
            classic: true,
            legacy_total_score: None,
            accuracy: None,
        };
        score.accuracy = Some(score.accuracy());
        performance(&score, &self.whole, self.mods).pp
    }

    pub fn reach(&mut self, counts: Counts, max_combo: u32) -> Reach {
        let clean = self.clean(counts);
        let judged = self.judged(counts);
        if judged == 0 {
            return Reach { now: 0.0, clean };
        }
        let mods = self.mods;
        let passed = self.passed(judged).clone();
        let mut score = Score {
            max_combo: max_combo.min(passed.max_combo),
            great: u32::from(counts.n300),
            ok: u32::from(counts.n100),
            meh: u32::from(counts.n50),
            miss: u32::from(counts.miss),
            slider_tail_hit: passed.slider_count,
            large_tick_miss: 0,
            classic: true,
            legacy_total_score: None,
            accuracy: None,
        };
        score.accuracy = Some(score.accuracy());
        let now = performance(&score, &passed, mods).pp;
        Reach { now: if now.is_finite() { now.max(0.0) } else { 0.0 }, clean }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus(name: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dossier-assay/corpus/maps").join(name)).unwrap()
    }

    fn all_great(objects: usize) -> Counts {
        Counts { n300: objects as u16, ..Counts::default() }
    }

    #[test]
    fn pp_grows_through_a_clean_play_and_ends_at_the_value_of_the_whole_map() {
        let mut pace = Pace::of(&corpus("5114204.osu"), 0).expect("a standard map");
        let objects = pace.objects();
        assert_eq!(pace.reach(Counts::default(), 0).now, 0.0, "nothing is judged yet");
        let mut last = 0.0;
        for share in [0.25, 0.5, 0.75, 1.0] {
            let judged = (objects as f64 * share) as usize;
            let part = pace.passed(judged).max_combo;
            let reach = pace.reach(all_great(judged), part);
            assert!(reach.now > last, "{share}: {} after {last}", reach.now);
            last = reach.now;
        }
        let whole = pace.reach(all_great(objects), pace.whole.max_combo);
        assert!((whole.now - whole.clean).abs() < 0.01, "a clean full play is worth the clean value: {whole:?}");
    }

    #[test]
    fn misses_cost_pp_now_and_the_clean_value_counts_only_the_hundreds_and_fifties() {
        let mut pace = Pace::of(&corpus("5114204.osu"), 0).unwrap();
        let half = pace.objects() / 2;
        let combo = pace.passed(half).max_combo;
        let clean = pace.reach(all_great(half), combo);
        let missed = pace.reach(Counts { n300: half as u16 - 6, miss: 6, ..Counts::default() }, combo / 3);
        assert!(missed.now < clean.now * 0.9, "{missed:?} against {clean:?}");
        assert!((missed.clean - clean.clean).abs() < 0.01, "misses do not lower what a clean play would give");
        let sloppy = pace.reach(Counts { n300: half as u16 - 40, n100: 40, ..Counts::default() }, combo);
        assert!(sloppy.clean < clean.clean);
    }

    #[test]
    fn mods_change_the_value_and_a_map_of_another_ruleset_has_none() {
        let text = corpus("5114204.osu");
        let plain = Pace::of(&text, 0).unwrap().clean(Counts::default());
        let hidden_double = Pace::of(&text, 8 | 64).unwrap().clean(Counts::default());
        assert!(hidden_double > plain * 1.3, "{hidden_double} against {plain}");
        assert!(Pace::of(&text.replace("Mode: 0", "Mode: 3").replace("Mode:0", "Mode:3"), 0).is_none() || !text.contains("Mode"));
        assert!(Pace::of("not a map", 0).is_none());
    }

    #[test]
    fn measuring_the_passed_part_is_quick_enough_to_do_during_play() {
        let mut slowest = std::time::Duration::ZERO;
        for name in ["5114204.osu", "3441410.osu", "1355247.osu"] {
            let mut pace = Pace::of(&corpus(name), 64).unwrap();
            let objects = pace.objects();
            for share in [0.3, 0.6, 0.9] {
                let judged = (objects as f64 * share) as usize;
                let begun = std::time::Instant::now();
                let _ = pace.reach(all_great(judged), 100);
                slowest = slowest.max(begun.elapsed());
            }
        }
        eprintln!("slowest {slowest:?}");
        assert!(slowest < std::time::Duration::from_millis(if cfg!(debug_assertions) { 1500 } else { 150 }), "{slowest:?}");
    }
}
