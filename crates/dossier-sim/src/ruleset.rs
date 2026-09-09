#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Stable,

    Lazer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ruleset {
    client: Client,

    legacy_note_lock: bool,

    pub relax: bool,

    spun_out: bool,

    whole_sliders: bool,

    head_carries_verdict: bool,

    legacy_health: bool,

    multipliers: crate::multiplier::Generation,
}

const FIRST_LAZER_VERSION: i32 = 30_000_000;

impl Ruleset {
    pub const STABLE: Self = Self {
        client: Client::Stable,
        relax: false,
        spun_out: false,
        legacy_note_lock: true,
        whole_sliders: true,
        head_carries_verdict: false,
        legacy_health: true,
        multipliers: crate::multiplier::Generation::V2,
    };

    pub const LAZER: Self = Self {
        client: Client::Lazer,
        relax: false,
        spun_out: false,
        legacy_note_lock: false,
        whole_sliders: false,
        head_carries_verdict: true,
        legacy_health: false,
        multipliers: crate::multiplier::Generation::V2,
    };

    pub fn of_replay_version(game_version: i32) -> Self {
        if game_version >= FIRST_LAZER_VERSION {
            Self::LAZER
        } else {
            Self::STABLE
        }
    }

    pub fn of_replay(replay: &dossier_replay::Replay) -> Self {
        let mut ruleset = Self::of_replay_version(replay.game_version);

        ruleset.relax = replay.mods.contains(dossier_replay::bits::RELAX);
        ruleset.spun_out = replay.mods.contains(dossier_replay::bits::SPUN_OUT);

        if ruleset.client == Client::Stable && replay.mods.contains(dossier_replay::bits::SCORE_V2)
        {
            ruleset.head_carries_verdict = true;
        }

        if ruleset.client == Client::Lazer {
            ruleset.multipliers =
                crate::multiplier::Generation::of_replay_version(replay.game_version);
        }
        if let Some(classic) = replay.lazer_mods().iter().find(|m| m.acronym == "CL") {
            ruleset.legacy_note_lock = classic.switch("classic_note_lock", true);
            ruleset.whole_sliders = classic.switch("no_slider_head_accuracy", true);

            ruleset.head_carries_verdict = !ruleset.whole_sliders;
            ruleset.legacy_health = classic.switch("classic_health", true);
        }
        ruleset
    }

    pub fn client(self) -> Client {
        self.client
    }

    pub fn multipliers(self) -> crate::multiplier::Generation {
        self.multipliers
    }

    pub fn legacy_health(self) -> bool {
        self.legacy_health
    }

    pub fn name(self) -> &'static str {
        match (self.client, self.legacy_note_lock || self.whole_sliders) {
            (Client::Stable, _) => "stable",
            (Client::Lazer, false) => "lazer",
            (Client::Lazer, true) => "lazer (classic)",
        }
    }

    pub fn spinner_swallows_presses(self) -> bool {
        self.client == Client::Stable
    }

    pub fn spin(self) -> crate::judge::Spin {
        crate::judge::Spin {
            spun_out: self.spun_out,
            relax: self.relax,
            smoothed: self.client == Client::Stable,
            rate: 1.0,
        }
    }

    pub fn spinner_counts_half_turns(self) -> bool {
        self.client == Client::Stable
    }

    pub fn blocks(
        self,
        blocker_end_ms: f64,
        blocker_start_ms: f64,
        target_start_ms: f64,
        press_time_ms: f64,
    ) -> bool {
        if self.legacy_note_lock {
            blocker_end_ms + STABLE_NOTELOCK_SLACK_MS < target_start_ms
        } else {
            press_time_ms < blocker_start_ms
        }
    }

    pub fn blocker_is_the_last_one(self) -> bool {
        !self.legacy_note_lock
    }

    pub fn can_block(self, is_spinner: bool) -> bool {
        !self.blocker_is_the_last_one() || !is_spinner
    }

    pub fn slider_is_scored_by_its_head(self) -> bool {
        !self.whole_sliders
    }

    pub fn slider_verdict_from_head(self) -> bool {
        self.head_carries_verdict
    }

    pub fn slider_verdict_also_needs_its_pieces(self) -> bool {
        self.client == Client::Stable && self.head_carries_verdict
    }

    pub fn writes_off_stranded_notes(self) -> bool {
        !self.legacy_note_lock
    }

    pub fn slider_swallows_notes_beneath(self) -> bool {
        self.legacy_note_lock
    }

    pub fn hittable_range_ms(self) -> f64 {
        400.0
    }
}

const STABLE_NOTELOCK_SLACK_MS: f64 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_version_picks_the_ruleset() {
        assert_eq!(Ruleset::of_replay_version(20_260_412), Ruleset::STABLE);
        assert_eq!(Ruleset::of_replay_version(20_230_206), Ruleset::STABLE);
        assert_eq!(Ruleset::of_replay_version(30_000_016), Ruleset::LAZER);
        assert_eq!(Ruleset::of_replay_version(30_000_018), Ruleset::LAZER);
    }

    fn replay_with(version: i32, mods: Vec<dossier_replay::LazerMod>) -> dossier_replay::Replay {
        dossier_replay::Replay {
            mode: dossier_replay::GameMode::Standard,
            game_version: version,
            beatmap_hash: String::new(),
            player: String::new(),
            replay_hash: String::new(),
            hits: dossier_replay::HitCounts::default(),
            score: 0,
            max_combo: 0,
            perfect_combo: false,
            mods: dossier_replay::Mods::new(0),
            life_bar: String::new(),
            timestamp_ticks: 0,
            online_score_id: 0,
            target_practice_accuracy: None,
            frames: Vec::new(),
            rng_seed: None,
            score_info: (!mods.is_empty()).then(|| dossier_replay::ScoreInfo {
                mods,
                ..dossier_replay::ScoreInfo::default()
            }),
        }
    }

    fn classic(settings: &[(&str, bool)]) -> dossier_replay::LazerMod {
        dossier_replay::LazerMod {
            acronym: "CL".into(),
            settings: settings
                .iter()
                .map(|(k, v)| ((*k).to_owned(), dossier_replay::Setting::Bool(*v)))
                .collect(),
        }
    }

    #[test]
    fn the_classic_mod_puts_stables_rules_back_one_at_a_time() {
        let all_on = Ruleset::of_replay(&replay_with(30_000_016, vec![classic(&[])]));
        assert!(all_on.slider_swallows_notes_beneath(), "note lock restored");
        assert!(
            !all_on.slider_is_scored_by_its_head(),
            "sliders whole again"
        );
        assert!(all_on.legacy_health());

        assert_eq!(all_on.client(), Client::Lazer);

        let no_sliders = Ruleset::of_replay(&replay_with(
            30_000_016,
            vec![classic(&[("no_slider_head_accuracy", false)])],
        ));
        assert!(no_sliders.slider_swallows_notes_beneath(), "lock still on");
        assert!(
            no_sliders.slider_is_scored_by_its_head(),
            "but sliders are lazer's again"
        );

        let no_lock = Ruleset::of_replay(&replay_with(
            30_000_016,
            vec![classic(&[("classic_note_lock", false)])],
        ));
        assert!(!no_lock.slider_swallows_notes_beneath());
        assert!(
            !no_lock.slider_is_scored_by_its_head(),
            "sliders still whole"
        );
    }

    fn stable_replay_with_mods(mods: u32) -> dossier_replay::Replay {
        let mut replay = replay_with(20_260_412, Vec::new());
        replay.mods = dossier_replay::Mods::new(mods);
        replay
    }

    #[test]
    fn score_v2_moves_a_stable_sliders_verdict_and_nothing_about_its_tracking() {
        let v2 = Ruleset::of_replay(&stable_replay_with_mods(dossier_replay::bits::SCORE_V2));
        assert_eq!(v2.client(), Client::Stable);
        assert!(
            v2.slider_verdict_from_head(),
            "the head decides the verdict"
        );
        assert!(
            !v2.slider_is_scored_by_its_head(),
            "and the slide is still tracked stable's way"
        );

        assert!(v2.legacy_health());
        assert!(v2.slider_swallows_notes_beneath());
    }

    #[test]
    fn a_stable_replay_without_score_v2_keeps_whole_sliders() {
        let plain = Ruleset::of_replay(&stable_replay_with_mods(0));
        assert!(!plain.slider_verdict_from_head());
        assert_eq!(plain, Ruleset::STABLE);

        let hidden = Ruleset::of_replay(&stable_replay_with_mods(dossier_replay::bits::HIDDEN));
        assert!(!hidden.slider_verdict_from_head());
    }

    #[test]
    fn score_v2_on_a_lazer_replay_changes_nothing() {
        let mut replay = replay_with(30_000_017, Vec::new());
        replay.mods = dossier_replay::Mods::new(dossier_replay::bits::SCORE_V2);
        assert_eq!(Ruleset::of_replay(&replay), Ruleset::LAZER);
    }

    #[test]
    fn a_lazer_replay_without_classic_keeps_lazers_rules() {
        let plain = Ruleset::of_replay(&replay_with(30_000_016, Vec::new()));
        assert_eq!(plain.client(), Client::Lazer);
        assert!(!plain.slider_swallows_notes_beneath());
        assert!(plain.slider_is_scored_by_its_head());
        assert!(!plain.legacy_health());

        let stable = Ruleset::of_replay(&replay_with(20_260_412, Vec::new()));
        assert_eq!(stable, Ruleset::STABLE);
    }

    #[test]
    fn the_replays_age_picks_which_multipliers_scored_it() {
        use crate::multiplier::Generation;
        let before = Ruleset::of_replay(&replay_with(30_000_016, Vec::new()));
        let after = Ruleset::of_replay(&replay_with(30_000_017, Vec::new()));
        assert_eq!(before.client(), after.client());
        assert_eq!(before.multipliers(), Generation::V1);
        assert_eq!(after.multipliers(), Generation::V2);
    }

    #[test]
    fn stable_blocks_a_late_press_where_lazer_does_not() {
        let (blocker_end, blocker_start, target_start, press) =
            (64_390.0, 64_390.0, 64_473.0, 64_427.0);
        assert!(Ruleset::STABLE.blocks(blocker_end, blocker_start, target_start, press));
        assert!(!Ruleset::LAZER.blocks(blocker_end, blocker_start, target_start, press));
    }

    #[test]
    fn lazer_blocks_a_press_that_arrives_before_the_blocker_is_due() {
        assert!(Ruleset::LAZER.blocks(64_390.0, 64_390.0, 64_473.0, 64_100.0));
    }

    #[test]
    fn stable_ignores_a_blocker_that_overlaps_its_target() {
        assert!(!Ruleset::STABLE.blocks(1_000.0, 1_000.0, 1_000.0, 1_000.0));
        assert!(!Ruleset::STABLE.blocks(1_000.0, 1_000.0, 1_002.0, 1_000.0));
        assert!(Ruleset::STABLE.blocks(1_000.0, 1_000.0, 1_004.0, 1_000.0));
    }
}
