use dossier_sim::{MissContext, Verification};

use dossier_replay::HitCounts;

pub struct Header {
    pub replay_path: String,
    pub client: String,
    pub player: String,
    pub mode: String,
    pub mods: String,
    pub beatmap_hash: String,
    pub counts: HitCounts,
    pub max_combo: u32,
    pub frames: usize,
    pub duration_ms: i64,

    pub lazer_mods: Vec<String>,
    pub statistics: Vec<(String, i64)>,
}

impl Header {
    pub fn human(&self) -> String {
        format!(
            "── {}\n   player  {}   mode {}   mods {}   client {}\n   map     {}\n   score   {}/{}/{}/{}  {}x  {:.2}%\n   frames  {} over {:.1}s\n",
            self.replay_path,
            self.player,
            self.mode,
            self.mods,
            self.client,
            self.beatmap_hash,
            self.counts.count_300,
            self.counts.count_100,
            self.counts.count_50,
            self.counts.count_miss,
            self.max_combo,
            self.counts.accuracy_std(),
            self.frames,
            self.duration_ms as f64 / 1000.0,
        ) + &self.lazer_lines()
    }

    fn lazer_lines(&self) -> String {
        let mut out = String::new();
        if !self.lazer_mods.is_empty() {
            out.push_str(&format!("   mods*   {}\n", self.lazer_mods.join(" ")));
        }
        if !self.statistics.is_empty() {
            let shown: Vec<String> = self
                .statistics
                .iter()
                .map(|(name, count)| format!("{name} {count}"))
                .collect();
            out.push_str(&format!("   judged  {}\n", shown.join("  ")));
        }
        out
    }

    pub fn json(&self) -> String {
        format!(
            concat!(
                "{{\"replay\":{},\"player\":{},\"mode\":{},\"mods\":{},\"beatmap_hash\":{},",
                "\"counts\":{},\"max_combo\":{},\"accuracy\":{:.4},\"frames\":{},\"duration_ms\":{}}}"
            ),
            quote(&self.replay_path),
            quote(&self.player),
            quote(&self.mode),
            quote(&self.mods),
            quote(&self.beatmap_hash),
            counts_json(self.counts),
            self.max_combo,
            self.counts.accuracy_std(),
            self.frames,
            self.duration_ms,
        )
    }
}

pub struct Report {
    pub replay_path: String,

    pub client: String,
    pub map_source: String,

    pub beatmap_md5: String,
    pub title: String,
    pub player: String,
    pub mods: String,
    pub objects: usize,
    pub check: Verification,
    pub our_accuracy: f64,
    pub their_accuracy: f64,
    pub misses: Vec<MissContext>,

    pub lenient_tails: usize,

    pub tails_near_the_rim: usize,

    pub max_possible_combo: u32,

    pub combo_chains: Vec<dossier_sim::ComboChain>,

    pub combo_suspects: Vec<dossier_sim::Suspect>,

    pub presses: dossier_sim::PressSummary,

    pub press_detail: Vec<dossier_sim::PressDetail>,

    pub window_50: f64,

    pub parts: Vec<PartCheck>,

    pub score_error: Option<f64>,
}

pub struct PartCheck {
    pub name: String,
    pub ours: i64,
    pub theirs: i64,
}

pub struct MissSummary {
    pub circles: usize,
    pub sliders: usize,
    pub spinners: usize,

    pub with_nearby_click: usize,

    pub geometry_suspects: usize,

    pub median_overshoot_px: Option<f64>,

    pub spin_rotations: Option<f64>,
    pub spin_required: Option<f64>,
}

impl MissSummary {
    pub fn of(misses: &[MissContext]) -> Self {
        let mut overshoots: Vec<f64> = misses
            .iter()
            .filter(|m| m.looks_like_a_geometry_error())
            .filter_map(|m| m.press_distance_px.map(|d| d - m.radius_px))
            .collect();
        overshoots.sort_by(f64::total_cmp);

        Self {
            circles: misses.iter().filter(|m| m.kind == "circle").count(),
            sliders: misses.iter().filter(|m| m.kind == "slider").count(),
            spinners: misses.iter().filter(|m| m.kind == "spinner").count(),
            with_nearby_click: misses.iter().filter(|m| m.press_dt_ms.is_some()).count(),
            geometry_suspects: overshoots.len(),
            median_overshoot_px: overshoots.get(overshoots.len() / 2).copied(),
            spin_rotations: mean(misses.iter().filter_map(|m| m.spin_rotations)),
            spin_required: mean(misses.iter().filter_map(|m| m.spin_required)),
        }
    }

    fn json(&self) -> String {
        format!(
            concat!(
                "{{\"circle\":{},\"slider\":{},\"spinner\":{},\"with_nearby_click\":{},",
                "\"geometry_suspects\":{},\"median_overshoot_px\":{},",
                "\"spin_rotations\":{},\"spin_required\":{}}}"
            ),
            self.circles,
            self.sliders,
            self.spinners,
            self.with_nearby_click,
            self.geometry_suspects,
            number(self.median_overshoot_px),
            number(self.spin_rotations),
            number(self.spin_required),
        )
    }
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let collected: Vec<f64> = values.collect();
    if collected.is_empty() {
        return None;
    }
    Some(collected.iter().sum::<f64>() / collected.len() as f64)
}

fn number(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.2}"),
        None => "null".to_owned(),
    }
}

impl Report {
    pub fn is_exact(&self) -> bool {
        self.check.is_exact()
    }

    fn parts_block(&self) -> String {
        if self.parts.is_empty() {
            return String::new();
        }
        let mut out = String::from("\n   lazer counted each judgement type:\n");
        for check in &self.parts {
            let flag = if check.ours == check.theirs { " " } else { "!" };
            out.push_str(&format!(
                "      {:<18} {:>6} {:>9}  {flag}\n",
                check.name, check.ours, check.theirs
            ));
        }
        out
    }

    pub fn human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("── {}\n", self.replay_path));
        out.push_str(&format!("   map     {}\n", self.title));
        out.push_str(&format!("   file    {}\n", self.map_source));
        out.push_str(&format!(
            "   player  {}   mods {}   objects {}\n   client  {}\n\n",
            self.player, self.mods, self.objects, self.client
        ));

        let ours = self.check.ours;
        let theirs = self.check.theirs;
        out.push_str(&self.incomplete_play());
        out.push_str("             ours    replay\n");
        for (label, a, b) in [
            (
                "300",
                u32::from(ours.count_300),
                u32::from(theirs.count_300),
            ),
            (
                "100",
                u32::from(ours.count_100),
                u32::from(theirs.count_100),
            ),
            ("50", u32::from(ours.count_50), u32::from(theirs.count_50)),
            (
                "miss",
                u32::from(ours.count_miss),
                u32::from(theirs.count_miss),
            ),
            (
                "combo",
                self.check.our_max_combo,
                self.check.their_max_combo,
            ),
        ] {
            let mark = if a == b { ' ' } else { '!' };
            out.push_str(&format!("   {label:>6} {a:>8} {b:>9}  {mark}\n"));
        }
        let acc_mark = if (self.our_accuracy - self.their_accuracy).abs() < 0.005 {
            ' '
        } else {
            '!'
        };
        out.push_str(&format!(
            "   {:>6} {:>7.2}% {:>8.2}%  {acc_mark}\n\n",
            "acc", self.our_accuracy, self.their_accuracy
        ));

        out.push_str(&format!(
            "   full combo would be {} by our count\n",
            self.max_possible_combo
        ));
        out.push_str(&self.parts_block());
        out.push_str(&self.combo_runs());
        out.push_str(&self.combo_split());
        out.push_str(&self.early_break());
        out.push_str(&format!("   {}\n", self.verdict()));
        out
    }

    fn combo_runs(&self) -> String {
        let (ours, theirs) = (self.check.our_max_combo, self.check.their_max_combo);
        if ours == theirs || self.combo_chains.is_empty() {
            return String::new();
        }
        let mut out = format!("   our combo runs, longest first (theirs peaks at {theirs}):\n");
        for chain in self.combo_chains.iter().take(4) {
            let ended = match (chain.ended_at_ms.is_finite(), chain.part) {
                (true, Some(part)) => format!(
                    "ended at {:.0}ms on object #{}, its {part:?}",
                    chain.ended_at_ms, chain.object_index
                ),
                (true, None) => format!(
                    "ended at {:.0}ms on object #{}",
                    chain.ended_at_ms, chain.object_index
                ),
                (false, _) => "ran to the end of the play".to_owned(),
            };
            let over = if chain.length > theirs {
                format!("  ← {} longer than theirs", chain.length - theirs)
            } else {
                String::new()
            };
            out.push_str(&format!("      {:>5}  {ended}{over}\n", chain.length));
        }
        out
    }

    fn combo_split(&self) -> String {
        let (ours, theirs) = (self.check.our_max_combo, self.check.their_max_combo);
        if ours <= theirs || self.combo_chains.is_empty() {
            return String::new();
        }
        let mut out = String::new();

        let generous =
            u32::from(self.check.ours.count_300).saturating_sub(self.check.theirs.count_300.into());
        if !self.combo_suspects.is_empty() {
            if generous > 1 {
                out.push_str(&format!(
                    "   we scored {generous} objects above the game, so it may have broken more\n   than once — these two are only the answer if it broke once:\n"
                ));
            } else {
                out.push_str("   the game's break has to be at one of:\n");
            }
            for s in &self.combo_suspects {
                let click = match (s.press_dt_ms, s.press_distance_px) {
                    (Some(dt), Some(distance)) => format!(
                        "click {dt:+.0}ms, {distance:.1}px from centre (radius {:.1})",
                        s.radius_px
                    ),
                    _ => "no click near it".to_owned(),
                };
                out.push_str(&format!(
                    "      #{} {} at {:.0}ms — we said {:?}, {click}\n",
                    s.object_index, s.kind, s.time_ms, s.ours
                ));
            }
        }
        out
    }

    fn early_break(&self) -> String {
        let (ours, theirs) = (self.check.our_max_combo, self.check.their_max_combo);
        let Some(longest) = self.combo_chains.first() else {
            return String::new();
        };
        if ours >= theirs || longest.length != ours {
            return String::new();
        }

        let describe = |chain: &dossier_sim::ComboChain| match chain.part {
            Some(part) => format!(
                "object #{} at {:.0}ms, on its {part:?}",
                chain.object_index, chain.ended_at_ms
            ),
            None => "the end of the play — nothing broke it".to_owned(),
        };

        let before = self
            .combo_chains
            .iter()
            .filter(|c| c.ended_at_ms < longest.ended_at_ms)
            .max_by(|a, b| a.ended_at_ms.total_cmp(&b.ended_at_ms));
        let mut out = format!(
            "   we broke {} time(s) the game did not — our longest run is {ours} to its {theirs}.\n",
            theirs - ours
        );
        if theirs - ours == 1 && self.check.counts_match() {
            match before {
                Some(before) => {
                    out.push_str(
                        "   If their run covers ours, the extra break is at one of its ends:\n",
                    );
                    out.push_str(&format!("      ours ended on {}\n", describe(longest)));
                    out.push_str(&format!(
                        "      the run before ours ended on {}\n",
                        describe(before)
                    ));
                }
                None => {
                    out.push_str(&format!(
                        "   Ours runs from the first object, so the break is where it ended:\n      {}\n",
                        describe(longest)
                    ));
                }
            }
        }
        out
    }

    fn incomplete_play(&self) -> String {
        if self.check.finished() {
            return String::new();
        }
        format!(
            "   this play ended early — {} of {} objects. Both columns below\n   \
             count only those, so the rest of the map is out of the comparison.\n\n",
            self.check.judged, self.check.objects
        )
    }

    pub fn trace(&self, window: Option<(f64, f64)>) -> String {
        let p = &self.presses;
        if p.total() == 0 {
            return "   no presses to account for\n".to_owned();
        }
        let mut out = format!("   {} presses:\n", p.total());
        for (count, label) in [
            (p.landed, "landed"),
            (p.took_a_note_early, "took a note early"),
            (p.refused, "refused by the lock"),
            (p.out_of_range, "out of range"),
            (p.ignored, "ignored, stacked predecessor"),
            (p.found_nothing, "found nothing under the cursor"),
        ] {
            if count > 0 {
                out.push_str(&format!(
                    "      {count:>6}  {label} ({:.1}%)\n",
                    count as f64 / p.total() as f64 * 100.0
                ));
            }
        }
        if !p.refusal_runs.is_empty() {
            out.push_str("   the lock lost the thread at:\n");
            for (at, count) in p.refusal_runs.iter().take(8) {
                out.push_str(&format!(
                    "      {:>7.1}s  {count} clicks in a row\n",
                    at / 1000.0
                ));
            }
            if p.refusal_runs.len() > 8 {
                out.push_str(&format!(
                    "      …and {} more runs\n",
                    p.refusal_runs.len() - 8
                ));
            }
        }
        out.push_str(&self.presses_between(window));
        out
    }

    fn presses_between(&self, window: Option<(f64, f64)>) -> String {
        let Some((from, to)) = window else {
            return String::new();
        };
        let mut out = format!("   clicks between {from:.0}ms and {to:.0}ms:\n");
        let mut shown = 0;
        for press in self
            .press_detail
            .iter()
            .filter(|p| p.time_ms >= from && p.time_ms <= to)
        {
            let target = match (press.object_index, press.error_ms, press.distance_px) {
                (Some(index), Some(error), Some(distance)) => format!(
                    "#{index} at {:.0}ms — {error:+.0}ms, {distance:.1}px of {:.1}",
                    press.object_ms.unwrap_or_default(),
                    press.radius_px
                ),
                _ => "nothing".to_owned(),
            };

            let blocker = match (press.blocked_by, press.object_index) {
                (Some(blocked_by), Some(index)) => {
                    format!(" ← blocked by #{blocked_by}, {} back", index - blocked_by)
                }
                _ => String::new(),
            };
            out.push_str(&format!(
                "      {:>8.0}ms  {:<20}  {target}{blocker}\n",
                press.time_ms,
                press.verdict.name()
            ));
            shown += 1;
        }
        if shown == 0 {
            out.push_str("      none\n");
        }
        out
    }

    pub fn marginal(&self, count: usize) -> String {
        let mut rows: Vec<(f64, f64, f64, &dossier_sim::PressDetail)> = self
            .press_detail
            .iter()
            .filter(|p| matches!(p.verdict, dossier_sim::Verdict::Landed { .. }))
            .filter_map(|p| {
                let (error, distance) = (p.error_ms?, p.distance_px?);
                let room_time = (self.window_50 - error.abs()) / self.window_50;
                let room_space = (p.radius_px - distance) / p.radius_px;
                Some((room_time.min(room_space), room_time, room_space, p))
            })
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));

        let mut out =
            format!("   the {count} thinnest hits (room left, against window and radius):\n");
        for (room, room_time, room_space, p) in rows.iter().take(count) {
            out.push_str(&format!(
                "      #{:<5} at {:>8.0}ms  {:+5.0}ms of {:.0} ({:>5.1}%)   {:>5.1}px of {:.1} ({:>5.1}%)   margin {:>5.1}%\n",
                p.object_index.unwrap_or_default(),
                p.object_ms.unwrap_or_default(),
                p.error_ms.unwrap_or_default(),
                self.window_50,
                room_time * 100.0,
                p.distance_px.unwrap_or_default(),
                p.radius_px,
                room_space * 100.0,
                room * 100.0,
            ));
        }
        out
    }

    pub fn explain(&self) -> String {
        if self.misses.is_empty() {
            return "   no misses to explain\n".to_owned();
        }
        let mut out = String::from("   our misses:\n");
        for miss in &self.misses {
            let where_ = match (miss.spin_rotations, miss.spin_required) {
                (Some(done), Some(needed)) => {
                    format!(
                        "{done:.1} of {needed:.1} turns ({:.0}%)",
                        done / needed * 100.0
                    )
                }
                _ => match (miss.press_dt_ms, miss.press_distance_px) {
                    (Some(dt), Some(distance)) => format!(
                        "click {dt:+.0}ms, {distance:.1}px from centre (radius {:.1}){}",
                        miss.radius_px,
                        if miss.looks_like_a_geometry_error() {
                            "  ← just outside"
                        } else {
                            ""
                        }
                    ),
                    _ => "no click nearby".to_owned(),
                },
            };
            out.push_str(&format!(
                "   #{:<5} {:<8} {:>9.0}ms  {where_}\n",
                miss.object_index, miss.kind, miss.time_ms
            ));
        }

        let summary = MissSummary::of(&self.misses);
        out.push_str(&format!(
            "   {} miss(es): {} circle, {} slider, {} spinner; {} with a click nearby, {} just outside\n",
            self.misses.len(),
            summary.circles,
            summary.sliders,
            summary.spinners,
            summary.with_nearby_click,
            summary.geometry_suspects,
        ));
        out
    }

    fn verdict(&self) -> String {
        if self.is_exact() {
            return "exact match".to_owned();
        }
        let mut parts = Vec::new();
        if !self.check.counts_match() {
            parts.push(format!(
                "counts off by {}",
                diff_summary(self.check.ours, self.check.theirs)
            ));
        }
        if !self.check.combo_matches() {
            parts.push(format!(
                "combo {:+}",
                i64::from(self.check.our_max_combo) - i64::from(self.check.their_max_combo)
            ));
        }
        format!("MISMATCH: {}", parts.join(", "))
    }

    pub fn json(&self) -> String {
        let ours = self.check.ours;
        let theirs = self.check.theirs;
        format!(
            concat!(
                "{{\"replay\":{},\"map_source\":{},\"title\":{},\"player\":{},\"mods\":{},",
                "\"client\":{},",
                "\"objects\":{},\"exact\":{},\"counts_match\":{},\"combo_match\":{},",
                "\"ours\":{},\"theirs\":{},",
                "\"our_max_combo\":{},\"their_max_combo\":{},",
                "\"our_accuracy\":{:.4},\"their_accuracy\":{:.4},\"misses\":{},",
                "\"lenient_tails\":{},\"tails_near_the_rim\":{},",
                "\"judged\":{},\"finished\":{},",
                "\"score_error\":{},",
                "\"max_possible_combo\":{}}}"
            ),
            quote(&self.replay_path),
            quote(&self.map_source),
            quote(&self.title),
            quote(&self.player),
            quote(&self.mods),
            quote(&self.client),
            self.objects,
            self.is_exact(),
            self.check.counts_match(),
            self.check.combo_matches(),
            counts_json(ours),
            counts_json(theirs),
            self.check.our_max_combo,
            self.check.their_max_combo,
            self.our_accuracy,
            self.their_accuracy,
            MissSummary::of(&self.misses).json(),
            self.lenient_tails,
            self.tails_near_the_rim,
            self.check.judged,
            self.check.finished(),
            self.score_error
                .map_or_else(|| "null".to_owned(), |off| format!("{off:.4}")),
            self.max_possible_combo,
        )
    }
}

fn diff_summary(ours: HitCounts, theirs: HitCounts) -> String {
    let mut parts = Vec::new();
    for (label, a, b) in [
        ("300", ours.count_300, theirs.count_300),
        ("100", ours.count_100, theirs.count_100),
        ("50", ours.count_50, theirs.count_50),
        ("miss", ours.count_miss, theirs.count_miss),
    ] {
        let delta = i64::from(a) - i64::from(b);
        if delta != 0 {
            parts.push(format!("{label} {delta:+}"));
        }
    }
    parts.join(" ")
}

fn counts_json(counts: HitCounts) -> String {
    format!(
        "{{\"300\":{},\"100\":{},\"50\":{},\"miss\":{}}}",
        counts.count_300, counts.count_100, counts.count_50, counts.count_miss
    )
}

pub use dossier_produce::json::quote;

pub fn error_json(replay_path: &str, message: &str) -> String {
    format!(
        "{{\"replay\":{},\"error\":{}}}",
        quote(replay_path),
        quote(message)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use dossier_sim::Verification;

    fn sample() -> Report {
        let counts = HitCounts {
            count_300: 10,
            count_100: 2,
            count_50: 1,
            count_miss: 3,
            ..HitCounts::default()
        };
        Report {
            replay_path: "a.osr".into(),
            client: "stable 20260412".into(),
            map_source: "songs/1.osz → hard.osu".into(),
            beatmap_md5: "0".repeat(32),
            title: "Artist - Title [Insane]".into(),
            player: "tester".into(),
            mods: "HDHR".into(),
            objects: 16,
            our_accuracy: counts.accuracy_std(),
            their_accuracy: counts.accuracy_std(),
            check: Verification {
                ours: counts,
                theirs: counts,
                our_max_combo: 100,
                their_max_combo: 100,
                objects: 16,
                judged: 16,
            },
            misses: Vec::new(),
            lenient_tails: 0,
            tails_near_the_rim: 0,
            max_possible_combo: 0,
            combo_chains: Vec::new(),
            combo_suspects: Vec::new(),
            presses: dossier_sim::PressSummary::default(),
            press_detail: Vec::new(),
            window_50: 150.0,
            parts: Vec::new(),
            score_error: None,
        }
    }

    #[test]
    fn a_play_that_ended_early_says_how_far_it_got() {
        let mut report = sample();
        report.objects = 100;
        report.check.objects = 100;
        report.check.judged = 40;
        let text = report.human();
        assert!(text.contains("ended early — 40 of 100 objects"), "{text}");
        assert!(text.contains("out of the comparison"), "{text}");
    }

    #[test]
    fn a_complete_play_says_nothing_about_ending_early() {
        let text = sample().human();
        assert!(!text.contains("ended early"), "{text}");
    }

    fn miss(distance: f64, dt: Option<f64>) -> MissContext {
        MissContext {
            object_index: 0,
            kind: "circle",
            time_ms: 1000.0,
            press_dt_ms: dt,
            press_distance_px: dt.map(|_| distance),
            radius_px: 32.0,
            spin_rotations: None,
            spin_required: None,
        }
    }

    fn spinner(done: f64, needed: f64) -> MissContext {
        MissContext {
            object_index: 0,
            kind: "spinner",
            time_ms: 1000.0,
            press_dt_ms: None,
            press_distance_px: None,
            radius_px: 32.0,
            spin_rotations: Some(done),
            spin_required: Some(needed),
        }
    }

    #[test]
    fn failed_spinners_report_how_far_short_they_fell() {
        let summary = MissSummary::of(&[spinner(10.0, 20.0), spinner(14.0, 20.0)]);
        assert_eq!(summary.spinners, 2);
        assert_eq!(summary.spin_rotations, Some(12.0));
        assert_eq!(summary.spin_required, Some(20.0));

        assert_eq!(summary.with_nearby_click, 0);
        assert_eq!(summary.geometry_suspects, 0);
    }

    #[test]
    fn a_click_just_outside_the_circle_is_flagged_as_our_problem() {
        assert!(miss(35.0, Some(4.0)).looks_like_a_geometry_error());

        assert!(!miss(200.0, Some(4.0)).looks_like_a_geometry_error());

        assert!(!miss(35.0, Some(250.0)).looks_like_a_geometry_error());

        assert!(!miss(35.0, None).looks_like_a_geometry_error());
    }

    #[test]
    fn the_summary_separates_our_misses_from_the_players() {
        let summary = MissSummary::of(&[
            miss(34.0, Some(2.0)),
            miss(36.0, Some(-3.0)),
            miss(300.0, None),
        ]);
        assert_eq!(summary.circles, 3);
        assert_eq!(summary.with_nearby_click, 2);
        assert_eq!(summary.geometry_suspects, 2);

        assert_eq!(summary.median_overshoot_px, Some(4.0));
    }

    #[test]
    fn the_json_carries_every_key_the_bot_reads() {
        let json = sample().json();
        for key in [
            "\"exact\"",
            "\"player\"",
            "\"mods\"",
            "\"objects\"",
            "\"ours\"",
            "\"theirs\"",
            "\"our_max_combo\"",
            "\"their_max_combo\"",
            "\"our_accuracy\"",
            "\"their_accuracy\"",
            "\"300\"",
            "\"100\"",
            "\"50\"",
            "\"miss\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }

    #[test]
    fn a_mismatch_is_named_rather_than_just_flagged() {
        let mut report = sample();
        report.check.theirs.count_300 = 11;
        report.check.theirs.count_miss = 2;
        let text = report.human();
        assert!(text.contains("MISMATCH"), "{text}");
        assert!(text.contains("300 -1"), "{text}");
        assert!(text.contains("miss +1"), "{text}");
    }

    #[test]
    fn quoting_escapes_what_would_break_the_line() {
        assert_eq!(quote(r#"a "b" \c"#), r#""a \"b\" \\c""#);
        assert_eq!(quote("line\nbreak"), r#""line\nbreak""#);
    }
}
